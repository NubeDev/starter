//! Phase 3 stage 9 smoke 2 — `FlowAsService` driving an upstream
//! event broadcast into per-event flow runs, backed by the real
//! [`SqliteRunStore`] so `find_by_dedup_key` is the
//! index-backed SQL lookup (not the stage-8
//! `RecordingSpiStore` test fake).
//!
//! Contract from the job WORKFLOW per-stage table:
//!
//! > wrap a toy flow via FlowAsService whose event source is a
//! > tokio::sync::mpsc channel the test owns; push three events,
//! > assert three runs land in SqliteRunStore all reaching
//! > Finished; call `Service::stop` and assert clean drain with
//! > no leaked tokio task; second sub-case: push the same event
//! > twice with the same dedup key and assert exactly one runs
//! > row, the second … returns the first run's outcome, and a
//! > `FlowEvent::DedupShortCircuit` is emitted.
//!
//! Substitution: `FlowAsService` subscribes via a
//! [`tokio::sync::broadcast`] channel (the SPI's
//! `EventSubscriber` shape is broadcast — `mpsc::Receiver` is
//! `!Clone` and can't be re-subscribed per Service::start).
//! Stage 8 already locked this; the WORKFLOW prose pre-dates the
//! D-F3.5 broadcast resolution. Semantics are equivalent for this
//! smoke (single producer / single consumer / at-least-once).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, watch};
use tokio::time::{sleep, timeout};

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow_spi::flow::{FlowId, FlowRevisionId, RunStore as SpiRunStore};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_spi::Principal;
use starter_flow_surfaces::{EventSubscriber, FlowAsService, ServiceSeedAdapter};
use starter_spi::auth::Role;
use starter_spi::service::{Event, EventSink, Service, ServiceContext, SinkResult};
use starter_store_sqlite::flow::{SqliteRunStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral, Pool};

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

fn build_topology() -> Arc<FlowTopology> {
    let node = NodeId::new("com.acme.stage9.svc.node").unwrap();
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node,
        Arc::new(Identity {
            kind: KindId::new("starter.flow.stage9-svc-identity").unwrap(),
        }),
    );
    Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        behaviors,
    })
}

fn build_principal() -> Principal {
    Principal {
        subject: "stage9-svc-user".into(),
        role: Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

/// EventSink that resolves dedup keys from `payload.id`.
struct PayloadIdSink;

#[async_trait]
impl EventSink for PayloadIdSink {
    async fn emit(&self, _kind: &str, _payload: serde_json::Value) -> SinkResult<()> {
        Ok(())
    }
    fn dedup_key(&self, _kind: &str, payload: &serde_json::Value) -> Option<String> {
        payload
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }
}

async fn boot_sqlite() -> (Pool, Arc<dyn SpiRunStore>) {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let store: Arc<dyn SpiRunStore> = Arc::new(SqliteRunStore::new(pool.clone()));
    (pool, store)
}

fn build_service(
    engine: Arc<Engine>,
    service_id: &str,
) -> (FlowAsService, broadcast::Sender<Event>) {
    let (tx, _) = broadcast::channel::<Event>(64);
    let tx_for_sub = tx.clone();
    let subscriber: EventSubscriber = Arc::new(move || tx_for_sub.subscribe());

    let node = NodeId::new("com.acme.stage9.svc.node").unwrap();
    let in_slot = SlotRef::new(node.clone(), "in");
    let out_slot = SlotRef::new(node, "out");

    let seed_adapter: ServiceSeedAdapter = Arc::new(move |event: &Event| {
        vec![(in_slot.clone(), SlotValue::Json(event.payload.clone()))]
    });

    let svc = FlowAsService::builder()
        .flow_id(FlowId::new("com.acme.stage9.svc.flow").unwrap())
        .revision(FlowRevisionId::new())
        .topology(build_topology())
        .terminal_slots(vec![out_slot])
        .engine(engine)
        .service_id(KindId::new(service_id).unwrap())
        .name("stage9-flow-as-service")
        .description("identity flow exposed as a service")
        .event_sink(Arc::new(PayloadIdSink))
        .event_subscriber(subscriber)
        .seed_adapter(seed_adapter)
        .principal(build_principal())
        .build()
        .expect("FlowAsService build");
    (svc, tx)
}

fn make_ctx() -> (ServiceContext, watch::Sender<bool>) {
    let (tx, rx) = watch::channel(false);
    let ctx = ServiceContext::new(
        Arc::new(prometheus::Registry::new()),
        rx,
        Arc::new(PayloadIdSink) as Arc<dyn EventSink>,
    );
    (ctx, tx)
}

async fn count_runs(pool: &Pool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM runs")
        .fetch_one(pool.sqlx())
        .await
        .expect("count runs")
}

async fn count_finished_runs(pool: &Pool) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM runs WHERE status IN ('completed', 'failed', 'cancelled')",
    )
    .fetch_one(pool.sqlx())
    .await
    .expect("count finished runs")
}

// ---------------------------------------------------------------------------
// 1. Happy path — three events, three runs, all finished.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_events_drive_three_runs_via_sqlite_backed_service() {
    let (pool, sqlite_store) = boot_sqlite().await;
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph).with_run_store(sqlite_store));

    let (svc, tx) = build_service(engine, "starter.flow.stage9-svc-happy");
    let (ctx, shutdown_tx) = make_ctx();
    let handle = svc.start(ctx).await.expect("start");

    for i in 0..3 {
        let payload = serde_json::json!({"id": format!("evt-{i}"), "n": i});
        tx.send(Event::new("test.event", payload)).unwrap();
    }

    // Wait for all three runs to land + finish in SqliteRunStore.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let finished = count_finished_runs(&pool).await;
        if finished >= 3 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!(
                "expected 3 finished runs in SqliteRunStore, saw {finished} \
                 ({} total rows)",
                count_runs(&pool).await
            );
        }
        sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(count_runs(&pool).await, 3, "one run per distinct event");
    assert_eq!(
        count_finished_runs(&pool).await,
        3,
        "all three runs must reach a terminal status"
    );

    // Clean shutdown — no leaked worker task.
    let _ = shutdown_tx.send(true);
    timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("worker joined in time")
        .expect("worker did not panic")
        .expect("worker returned Ok");
}

// ---------------------------------------------------------------------------
// 2. D-F3.12 dedup short-circuit on re-delivery — the SqliteRunStore's
//    UNIQUE (service_name, dedup_key) partial index is the
//    correctness backstop; the `find_by_dedup_key` index lookup
//    must hit on re-delivery and the wrapper must short-circuit
//    without starting a second run.
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn re_delivered_event_short_circuits_via_sqlite_dedup_index() {
    let (pool, sqlite_store) = boot_sqlite().await;
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph).with_run_store(sqlite_store));

    let (svc, tx) = build_service(engine, "starter.flow.stage9-svc-dedup");
    let (ctx, shutdown_tx) = make_ctx();
    let handle = svc.start(ctx).await.expect("start");

    let event = || Event::new("test.event", serde_json::json!({"id": "dup-key-A"}));
    tx.send(event()).unwrap();

    // Wait for the first run to be recorded so the
    // `find_by_dedup_key` index has something to hit on
    // re-delivery.
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        if count_runs(&pool).await >= 1 {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("first run did not record before re-delivery window");
        }
        sleep(Duration::from_millis(10)).await;
    }

    // Re-deliver: the wrapper must short-circuit, NOT start a
    // second run.
    tx.send(event()).unwrap();
    sleep(Duration::from_millis(300)).await;

    assert_eq!(
        count_runs(&pool).await,
        1,
        "re-delivery must short-circuit; SqliteRunStore must have \
         exactly one row for this dedup key"
    );

    // The recorded run must carry the dedup key under the
    // service id — confirms the SPI wrote it on the
    // `RunStore::start` call site (stage-8 `with_dedup_key`).
    let recorded: (String, String) =
        sqlx::query_as("SELECT service_name, dedup_key FROM runs WHERE dedup_key IS NOT NULL")
            .fetch_one(pool.sqlx())
            .await
            .expect("fetch dedup row");
    assert_eq!(recorded.0, "starter.flow.stage9-svc-dedup");
    assert_eq!(recorded.1, "dup-key-A");

    let _ = shutdown_tx.send(true);
    timeout(Duration::from_secs(2), handle.join)
        .await
        .expect("worker joined in time")
        .expect("worker did not panic")
        .expect("worker returned Ok");
}
