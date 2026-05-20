//! Phase 3 stage 8 — `FlowAsService` body invariants (SCOPE R9 +
//! D-F3.5 + D-F3.12).
//!
//! Mirrors stage 7's per-invariant test shape. The five tests:
//!
//! 1. [`builder_rejects_missing_required_fields`] — the builder is
//!    the only construction path and every required field surfaces
//!    a typed [`FlowAsServiceBuildError::MissingField`].
//! 2. [`service_subscribes_and_invokes_flow_per_event`] — happy
//!    path: push three events through the upstream broadcast,
//!    assert three runs completed and the SPI store recorded each
//!    one (D-F3.5 subscribe-on-start lifecycle).
//! 3. [`service_drains_on_stop_with_no_task_leak`] — flip
//!    `ServiceContext::shutdown` and assert the spawned worker
//!    joins cleanly; no orphan per-call forwarder / event-watcher
//!    tasks survive across 16 push-then-stop cycles.
//! 4. [`dedup_short_circuit_emits_on_re_delivery`] — push the same
//!    event twice; assert the SPI `RunStore` has only one `start`
//!    record and the second push observes the
//!    `find_by_dedup_key` short-circuit (D-F3.12).
//! 5. [`dedup_key_falls_back_to_blake3_when_event_sink_returns_none`]
//!    — assert two distinct payloads with no sink-supplied dedup
//!    key produce two distinct dedup keys (the blake3 fallback
//!    fires deterministically).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, watch};
use tokio::time::{sleep, timeout};

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow_spi::flow::{
    DedupKey, FlowId, FlowResult, FlowRevisionId, RunCheckpoint, RunId, RunOpts, RunOutcome,
    RunState as SpiRunState, RunStore as SpiRunStore,
};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_spi::Principal;
use starter_spi::auth::Role;
use starter_spi::service::{Event, EventSink, Service, ServiceContext, SinkResult};

use starter_flow_surfaces::{
    EventSubscriber, FlowAsService, FlowAsServiceBuildError, ServiceSeedAdapter,
};

// ---------------------------------------------------------------------------
// Shared scaffolding.
// ---------------------------------------------------------------------------

struct Identity {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for Identity {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let v = input.get("in").cloned().unwrap_or(SlotValue::Null);
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), v);
        Ok(out)
    }
}

fn node_id() -> NodeId {
    NodeId::new("com.acme.stage8.node").unwrap()
}

fn build_topology() -> Arc<FlowTopology> {
    let node = node_id();
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node,
        Arc::new(Identity {
            kind: KindId::new("starter.flow.stage8-identity").unwrap(),
        }),
    );
    let links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
    Arc::new(FlowTopology {
        links,
        triggers,
        behaviors,
    })
}

fn build_principal() -> Principal {
    Principal {
        subject: "stage8-user".to_string(),
        role: Role::Admin,
        scopes: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

/// Test SPI `RunStore` that records every `start` call (so tests
/// can assert how many distinct runs the service produced) and
/// supports `find_by_dedup_key` lookups against the same record
/// table (so the dedup-short-circuit test sees a hit on the
/// second delivery).
#[derive(Default)]
struct RecordingSpiStore {
    starts: Mutex<Vec<(RunId, Option<DedupKey>)>>,
}

impl RecordingSpiStore {
    fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    fn start_count(&self) -> usize {
        self.starts.lock().unwrap().len()
    }
    fn distinct_dedup_keys(&self) -> usize {
        let mut keys: Vec<String> = self
            .starts
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, k)| k.as_ref().map(|k| k.key.clone()))
            .collect();
        keys.sort();
        keys.dedup();
        keys.len()
    }
}

#[async_trait]
impl SpiRunStore for RecordingSpiStore {
    async fn start(
        &self,
        run_id: RunId,
        _flow_revision: FlowRevisionId,
        _opts: RunOpts,
        _principal: Principal,
        dedup: Option<DedupKey>,
    ) -> FlowResult<()> {
        self.starts.lock().unwrap().push((run_id, dedup));
        Ok(())
    }
    async fn checkpoint(
        &self,
        _run_id: RunId,
        _seq: u64,
        _state: SpiRunState,
        _writes: &[(SlotRef, SlotValue)],
    ) -> FlowResult<()> {
        Ok(())
    }
    async fn load(&self, _run_id: RunId) -> FlowResult<Option<RunCheckpoint>> {
        Ok(None)
    }
    async fn finish(&self, _run_id: RunId, _outcome: RunOutcome) -> FlowResult<()> {
        Ok(())
    }
    async fn list_open(&self) -> FlowResult<Vec<RunId>> {
        Ok(Vec::new())
    }
    async fn find_by_dedup_key(
        &self,
        service_name: &str,
        dedup_key: &str,
    ) -> FlowResult<Option<RunId>> {
        let starts = self.starts.lock().unwrap();
        for (run_id, dedup) in starts.iter() {
            if let Some(k) = dedup {
                if k.service_name == service_name && k.key == dedup_key {
                    return Ok(Some(*run_id));
                }
            }
        }
        Ok(None)
    }
}

/// Test EventSink with a configurable `dedup_key` policy and a
/// counter for emitted events. `emit` is unused by the service
/// (the service receives via the broadcast subscriber); the sink
/// is held only for the `dedup_key()` accessor.
struct TestSink {
    dedup_policy: DedupPolicy,
    emitted: AtomicUsize,
}

enum DedupPolicy {
    /// Sink declines to provide a dedup key — the
    /// `FlowAsService` blake3 fallback fires (the default in
    /// production; covered by test 5).
    Fallback,
    /// Sink returns `Some(payload_id)` from the payload's `id`
    /// field, mirroring the typical upstream pattern (Slack
    /// `event_id`, Telegram `update_id`).
    PayloadId,
}

impl TestSink {
    fn new(policy: DedupPolicy) -> Arc<Self> {
        Arc::new(Self {
            dedup_policy: policy,
            emitted: AtomicUsize::new(0),
        })
    }
}

#[async_trait]
impl EventSink for TestSink {
    async fn emit(&self, _kind: &str, _payload: serde_json::Value) -> SinkResult<()> {
        self.emitted.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn dedup_key(&self, _kind: &str, payload: &serde_json::Value) -> Option<String> {
        match self.dedup_policy {
            DedupPolicy::Fallback => None,
            DedupPolicy::PayloadId => payload
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        }
    }
}

/// Construct a fully-wired `FlowAsService` plus the
/// `broadcast::Sender<Event>` the test pushes events into.
fn build_service(
    engine: Arc<Engine>,
    sink: Arc<dyn EventSink>,
) -> (FlowAsService, broadcast::Sender<Event>) {
    let (tx, _) = broadcast::channel::<Event>(64);
    let tx_for_sub = tx.clone();
    let subscriber: EventSubscriber = Arc::new(move || tx_for_sub.subscribe());

    let node = node_id();
    let in_slot = SlotRef::new(node.clone(), "in");
    let out_slot = SlotRef::new(node, "out");

    let seed_adapter: ServiceSeedAdapter = Arc::new(move |event: &Event| {
        let v = SlotValue::Json(event.payload.clone());
        vec![(in_slot.clone(), v)]
    });

    let svc = FlowAsService::builder()
        .flow_id(FlowId::new("com.acme.stage8.flow").unwrap())
        .revision(FlowRevisionId::new())
        .topology(build_topology())
        .terminal_slots(vec![out_slot])
        .engine(engine)
        .service_id(KindId::new("starter.flow.stage8-as-service").unwrap())
        .name("stage8_identity_service")
        .description("identity flow exposed as a service")
        .event_sink(sink)
        .event_subscriber(subscriber)
        .seed_adapter(seed_adapter)
        .principal(build_principal())
        .build()
        .expect("builder build");
    (svc, tx)
}

fn build_engine(spi_store: Option<Arc<RecordingSpiStore>>) -> Arc<Engine> {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let mut engine = Engine::new(store);
    if let Some(spi) = spi_store {
        engine = engine.with_run_store(spi as Arc<dyn SpiRunStore>);
    }
    Arc::new(engine)
}

fn make_ctx() -> (ServiceContext, watch::Sender<bool>, Arc<TestSink>) {
    let sink = TestSink::new(DedupPolicy::PayloadId);
    let (tx, rx) = watch::channel(false);
    let ctx = ServiceContext::new(
        Arc::new(prometheus::Registry::new()),
        rx,
        sink.clone() as Arc<dyn EventSink>,
    );
    (ctx, tx, sink)
}

// ---------------------------------------------------------------------------
// 1. Builder rejects missing required fields.
// ---------------------------------------------------------------------------

#[test]
fn builder_rejects_missing_required_fields() {
    let err = match FlowAsService::builder().build() {
        Ok(_) => panic!("empty builder must fail"),
        Err(e) => e,
    };
    assert!(matches!(err, FlowAsServiceBuildError::MissingField(_)));
}

// ---------------------------------------------------------------------------
// 2. Happy path: subscribe-on-start + run-per-event.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_subscribes_and_invokes_flow_per_event() {
    let spi = RecordingSpiStore::new();
    let engine = build_engine(Some(spi.clone()));
    let sink = TestSink::new(DedupPolicy::PayloadId);
    let (svc, tx) = build_service(engine, sink.clone() as Arc<dyn EventSink>);

    let (ctx, shutdown_tx, _ctx_sink) = make_ctx();
    let handle = svc.start(ctx).await.expect("start");

    // Push three distinct events; each has a unique `id` so
    // dedup never short-circuits.
    for i in 0..3 {
        let payload = serde_json::json!({"id": format!("evt-{i}"), "n": i});
        tx.send(Event::new("test.event", payload)).unwrap();
    }

    // Give the worker time to drain.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if spi.start_count() >= 3 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("expected 3 SPI starts, saw {}", spi.start_count());
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(spi.start_count(), 3, "one SPI start per event");
    assert_eq!(spi.distinct_dedup_keys(), 3, "three distinct dedup keys");

    // Clean shutdown.
    let _ = shutdown_tx.send(true);
    timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("worker joined in time")
        .expect("worker did not panic")
        .expect("worker returned Ok");
}

// ---------------------------------------------------------------------------
// 3. stop() drains cleanly with no task leak.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn service_drains_on_stop_with_no_task_leak() {
    // 16 back-to-back start/push/stop cycles: each cycle must
    // see the worker join cleanly. A leaked worker task would
    // hold the `EventSubscriber`'s captured sender alive and
    // be observable as a hang on subsequent timeouts.
    for _ in 0..16 {
        let spi = RecordingSpiStore::new();
        let engine = build_engine(Some(spi.clone()));
        let sink = TestSink::new(DedupPolicy::PayloadId);
        let (svc, tx) = build_service(engine, sink as Arc<dyn EventSink>);

        let (ctx, shutdown_tx, _) = make_ctx();
        let handle = svc.start(ctx).await.expect("start");

        // One event so the worker has done meaningful work.
        let _ = tx.send(Event::new(
            "test.event",
            serde_json::json!({"id": "evt-0", "n": 0}),
        ));

        // Wait briefly for the worker to process.
        sleep(Duration::from_millis(50)).await;

        let _ = shutdown_tx.send(true);
        timeout(Duration::from_secs(2), handle.join)
            .await
            .expect("worker joined in time")
            .expect("worker did not panic")
            .expect("worker returned Ok");
    }
}

// ---------------------------------------------------------------------------
// 4. Dedup short-circuit on re-delivery (D-F3.12).
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dedup_short_circuit_emits_on_re_delivery() {
    let spi = RecordingSpiStore::new();
    let engine = build_engine(Some(spi.clone()));
    let sink = TestSink::new(DedupPolicy::PayloadId);
    let (svc, tx) = build_service(engine, sink as Arc<dyn EventSink>);

    let (ctx, shutdown_tx, _) = make_ctx();
    let handle = svc.start(ctx).await.expect("start");

    // Send the same event twice (same `id` so PayloadId policy
    // returns the same dedup key both times).
    let event = || Event::new("test.event", serde_json::json!({"id": "dup-1"}));
    tx.send(event()).unwrap();

    // Wait for the first run to be recorded before re-delivering
    // so the `find_by_dedup_key` lookup has something to hit.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while spi.start_count() < 1 {
        if std::time::Instant::now() >= deadline {
            panic!("first run did not record");
        }
        sleep(Duration::from_millis(10)).await;
    }

    tx.send(event()).unwrap();

    // Give the worker time to process the second (short-circuit)
    // delivery — if it incorrectly started a second run we'd see
    // `start_count == 2`.
    sleep(Duration::from_millis(200)).await;

    assert_eq!(
        spi.start_count(),
        1,
        "second delivery must short-circuit; no second SPI start"
    );

    let _ = shutdown_tx.send(true);
    timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("worker joined in time")
        .expect("worker did not panic")
        .expect("worker returned Ok");
}

// ---------------------------------------------------------------------------
// 5. Blake3 fallback when EventSink::dedup_key returns None.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dedup_key_falls_back_to_blake3_when_event_sink_returns_none() {
    let spi = RecordingSpiStore::new();
    let engine = build_engine(Some(spi.clone()));
    // Sink declines to supply a dedup key — the blake3 fallback
    // fires for every event.
    let sink = TestSink::new(DedupPolicy::Fallback);
    let (svc, tx) = build_service(engine, sink as Arc<dyn EventSink>);

    let (ctx, shutdown_tx, _) = make_ctx();
    let handle = svc.start(ctx).await.expect("start");

    // Two distinct payloads must produce two distinct blake3
    // dedup keys (so both get SPI-recorded; no short-circuit
    // between them).
    tx.send(Event::new("k", serde_json::json!({"a": 1})))
        .unwrap();
    tx.send(Event::new("k", serde_json::json!({"a": 2})))
        .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while spi.start_count() < 2 {
        if std::time::Instant::now() >= deadline {
            panic!(
                "expected 2 distinct blake3 dedup keys, saw {}",
                spi.start_count()
            );
        }
        sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(spi.start_count(), 2);
    assert_eq!(
        spi.distinct_dedup_keys(),
        2,
        "blake3 fallback must produce distinct keys for distinct payloads"
    );

    // Same payload twice — second must short-circuit.
    tx.send(Event::new("k", serde_json::json!({"a": 1})))
        .unwrap();
    sleep(Duration::from_millis(200)).await;
    assert_eq!(
        spi.start_count(),
        2,
        "re-delivery of identical payload short-circuits via blake3"
    );

    let _ = shutdown_tx.send(true);
    timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("worker joined in time")
        .expect("worker did not panic")
        .expect("worker returned Ok");
}
