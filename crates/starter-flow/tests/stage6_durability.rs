//! Phase 3 stage 6 — durability hardening invariants (D-F3.10 +
//! D-F3.11).
//!
//! Per the job WORKFLOW per-stage table, stage 6 lands six items
//! and "unit tests cover each invariant in isolation". This file is
//! that coverage:
//!
//! 1. [`tick_counter_is_u64_sized`] — promotes the compile-time
//!    `const _: () = assert!(size_of::<TickCounter>() == 8)` to a
//!    visible runtime check so a refactor that silently widens /
//!    narrows the counter shows up red here too.
//! 2. [`engine_default_health_is_healthy_and_handle_round_trips`] —
//!    [`starter_flow::engine::Engine::health`] starts
//!    [`EngineHealth::Healthy`] and the shared handle round-trips
//!    `Healthy ↔ Degraded`.
//! 3. [`failing_run_store_emits_five_checkpoint_failed_events_then_degrades`]
//!    — the propagator's retry-with-backoff fires
//!    [`FlowEvent::CheckpointFailed`] once per attempt for attempts
//!    `1..=5` and flips the engine to
//!    [`EngineHealth::Degraded`] on the fifth.
//! 4. [`start_rejects_with_backend_unavailable_while_degraded`] —
//!    `FlowRunner::start` is the per-run rejection point;
//!    [`EngineError::BackendUnavailable`] is what callers see while
//!    the engine is degraded.
//! 5. [`degraded_queue_evict_oldest_increments_dropped_count`] —
//!    `RunOpts.degraded_queue_capacity = 1` plus a permanently-
//!    failing store: after multiple ticks the queue evicts oldest
//!    and `RunMetrics.degraded_dropped_count` is non-zero.
//! 6. [`engine_recovers_to_healthy_when_store_comes_back`] — a
//!    flaky store that errors `N` times then succeeds: the engine
//!    transitions back to `Healthy` on the first successful
//!    checkpoint write.
//! 7. [`lagged_subscriber_increments_subscriber_lagged_count`] —
//!    the engine's `Lagged`-watcher (`spawn_lagged_watcher`)
//!    increments the per-run metric on
//!    `broadcast::error::RecvError::Lagged(n)`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::broadcast;
use tokio::time::{sleep, timeout};

use starter_flow_spi::flow::{
    DedupKey, EngineError, EngineHealth, FlowError, FlowEvent, FlowId, FlowResult, FlowRevisionId,
    RunCheckpoint, RunId, RunOpts, RunOutcome, RunState as SpiRunState, RunStore as SpiRunStore,
};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_spi::Principal;

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::health::HealthHandle;
use starter_flow::metrics::{spawn_lagged_watcher, RunMetricsCell};
use starter_flow::propagator::{FlowTopology, TickCounter};
use starter_flow::run::{FlowRunner, FlowRunnerConfig, InMemoryRunStore, RunSpec};

/// Build a [`FlowRunnerConfig`] with a long quiescence window so
/// the coordinator does not declare the run complete while the
/// propagator is mid-`tokio::time::sleep` inside the
/// retry-with-backoff loop (the longest backoff is 800 ms; default
/// quiescence is 100 ms).
fn long_quiesce_config() -> FlowRunnerConfig {
    let mut cfg = FlowRunnerConfig::default();
    cfg.quiescence = Duration::from_secs(3);
    cfg
}

// ---------------------------------------------------------------------------
// Test topology: a single identity node triggered by `in`, writing
// to `out`. Each seed produces exactly one propagator tick =>
// exactly one checkpoint write.
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

fn build_spec(seed: SlotValue) -> RunSpec {
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    let node = NodeId::new("com.acme.stage6.identity").unwrap();
    let kind = KindId::new("starter.flow.stage6-identity").unwrap();
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node.clone(),
        Arc::new(Identity { kind }) as Arc<dyn NodeBehavior>,
    );
    let links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
    let topology = Arc::new(FlowTopology {
        links,
        triggers,
        behaviors,
    });
    let seeds = vec![(SlotRef::new(node.clone(), "in"), seed)];
    let terminal_slots = vec![SlotRef::new(node, "out")];
    RunSpec::new(
        FlowId::new("com.acme.stage6.flow").unwrap(),
        FlowRevisionId::new(),
        topology,
        seeds,
        terminal_slots,
    )
}

// ---------------------------------------------------------------------------
// FlakyRunStore: returns `Backend("flaky")` for the first
// `fail_until` checkpoint calls, then succeeds. Records every call.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct FlakyRunStore {
    fail_until: AtomicUsize,
    attempts: AtomicUsize,
    successes: Mutex<Vec<RunCheckpoint>>,
}

impl FlakyRunStore {
    fn new(fail_first_n: usize) -> Arc<Self> {
        Arc::new(Self {
            fail_until: AtomicUsize::new(fail_first_n),
            attempts: AtomicUsize::new(0),
            successes: Mutex::new(Vec::new()),
        })
    }

    fn attempts(&self) -> usize {
        self.attempts.load(Ordering::SeqCst)
    }
    fn success_count(&self) -> usize {
        self.successes.lock().unwrap().len()
    }
}

#[async_trait]
impl SpiRunStore for FlakyRunStore {
    async fn start(
        &self,
        _run_id: RunId,
        _flow_revision: FlowRevisionId,
        _opts: RunOpts,
        _principal: Principal,
        _dedup: Option<DedupKey>,
    ) -> FlowResult<()> {
        Ok(())
    }
    async fn checkpoint(
        &self,
        run_id: RunId,
        seq: u64,
        state: SpiRunState,
        writes: &[(SlotRef, SlotValue)],
    ) -> FlowResult<()> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
        let fail_until = self.fail_until.load(Ordering::SeqCst);
        if attempt <= fail_until {
            return Err(FlowError::Backend("flaky".into()));
        }
        self.successes.lock().unwrap().push(RunCheckpoint::new(
            run_id,
            seq,
            state,
            writes.to_vec(),
        ));
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
        _service_name: &str,
        _dedup_key: &str,
    ) -> FlowResult<Option<RunId>> {
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn tick_counter_is_u64_sized() {
    // The compile-time `const _: () = assert!(...)` in
    // `propagator.rs` is the real guard; this runtime check makes
    // the invariant visible to anyone reading the test list.
    assert_eq!(std::mem::size_of::<TickCounter>(), 8);
    // Increments are saturating and start at 0.
    let mut t = TickCounter::new();
    assert_eq!(t.get(), 0);
    assert_eq!(t.tick(), 1);
    assert_eq!(t.tick(), 2);
}

#[tokio::test]
async fn engine_default_health_is_healthy_and_handle_round_trips() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Engine::new(store);
    assert_eq!(engine.health(), EngineHealth::Healthy);
    let handle = engine.health_handle();
    handle.set_degraded();
    assert_eq!(engine.health(), EngineHealth::Degraded);
    handle.set_healthy();
    assert_eq!(engine.health(), EngineHealth::Healthy);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failing_run_store_emits_five_checkpoint_failed_events_then_degrades() {
    // Backend fails forever => the first tick exhausts all 5
    // retries, emitting one `CheckpointFailed` per attempt, then
    // degrades the shared health handle.
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let spi = FlakyRunStore::new(usize::MAX);
    let in_mem: Arc<dyn starter_flow::run::RunStore> = Arc::new(InMemoryRunStore::new());
    let health = HealthHandle::new();
    let runner = FlowRunner::new(store.clone(), in_mem)
        .with_spi_run_store(spi.clone())
        .with_health_handle(health.clone())
        .with_config(long_quiesce_config());

    let spec = build_spec(SlotValue::Int(42));
    let mut handle = runner
        .start(spec, SlotMap::new())
        .await
        .expect("start while healthy");

    // The single propagator tick goes through 5 retries with the
    // 50→100→200→400→800 ms backoff (~1.5s) before the engine
    // queues+degrades; the coordinator then waits for quiescence.
    // Bound the join generously.
    let _status = timeout(Duration::from_secs(8), &mut handle.join)
        .await
        .expect("run did not complete")
        .expect("coordinator panicked");

    // Drain the broadcast and bucket the CheckpointFailed attempts.
    let mut attempts: Vec<u32> = Vec::new();
    loop {
        match handle.initial_rx.try_recv() {
            Ok(FlowEvent::CheckpointFailed { attempt, .. }) => attempts.push(attempt),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    assert_eq!(
        attempts,
        vec![1, 2, 3, 4, 5],
        "expected one CheckpointFailed per attempt 1..=5, got {attempts:?}",
    );

    // Engine flipped to Degraded after the fifth failure.
    assert_eq!(health.get(), EngineHealth::Degraded);

    // The flaky store recorded at least 5 attempts.
    assert!(
        spi.attempts() >= 5,
        "expected >=5 store.checkpoint attempts, got {}",
        spi.attempts()
    );
}

#[tokio::test]
async fn start_rejects_with_backend_unavailable_while_degraded() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let in_mem: Arc<dyn starter_flow::run::RunStore> = Arc::new(InMemoryRunStore::new());
    let health = HealthHandle::new();
    let runner = FlowRunner::new(store, in_mem).with_health_handle(health.clone());

    health.set_degraded();
    let err = match runner
        .start(build_spec(SlotValue::Int(1)), SlotMap::new())
        .await
    {
        Ok(_) => panic!("start should reject while degraded"),
        Err(e) => e,
    };
    assert!(
        matches!(err, EngineError::BackendUnavailable),
        "expected BackendUnavailable, got {err:?}",
    );

    // Recovery: once health is back to Healthy, start succeeds.
    health.set_healthy();
    let _ok = runner
        .start(build_spec(SlotValue::Int(2)), SlotMap::new())
        .await
        .expect("start after recovery");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn degraded_queue_evict_oldest_increments_dropped_count() {
    // Single run with three back-to-back seed writes against a
    // permanently failing store and `degraded_queue_capacity = 1`.
    // Each tick's failed checkpoint batch pushes into the cap-1
    // queue; pushes 2 and 3 evict the head and bump
    // `RunMetrics.degraded_dropped_count`.
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let spi = FlakyRunStore::new(usize::MAX);
    let in_mem: Arc<dyn starter_flow::run::RunStore> = Arc::new(InMemoryRunStore::new());
    let mut opts = RunOpts::default();
    opts.degraded_queue_capacity = 1;
    let runner = FlowRunner::new(store.clone(), in_mem)
        .with_spi_run_store(spi)
        .with_run_opts(opts)
        .with_config(long_quiesce_config());

    // Three-seed spec: write `in` three times with distinct values
    // so each produces a SlotChanged tick.
    let node = NodeId::new("com.acme.stage6.identity").unwrap();
    let kind = KindId::new("starter.flow.stage6-identity").unwrap();
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node.clone(),
        Arc::new(Identity { kind }) as Arc<dyn NodeBehavior>,
    );
    let topology = Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        behaviors,
    });
    let seeds = vec![
        (SlotRef::new(node.clone(), "in"), SlotValue::Int(1)),
        (SlotRef::new(node.clone(), "in"), SlotValue::Int(2)),
        (SlotRef::new(node.clone(), "in"), SlotValue::Int(3)),
    ];
    let spec = RunSpec::new(
        FlowId::new("com.acme.stage6.flow").unwrap(),
        FlowRevisionId::new(),
        topology,
        seeds,
        vec![SlotRef::new(node, "out")],
    );
    let mut handle = runner
        .start(spec, SlotMap::new())
        .await
        .expect("start while healthy");
    let _ = timeout(Duration::from_secs(30), &mut handle.join).await;

    let snap = handle.metrics.snapshot();
    assert!(
        snap.degraded_dropped_count >= 1,
        "expected at least one degraded-queue eviction, got {}",
        snap.degraded_dropped_count
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn engine_recovers_to_healthy_when_store_comes_back() {
    // Store fails the first 5 checkpoint calls (exactly one full
    // retry cycle => engine degrades), then succeeds. The second
    // tick goes through cleanly and flips health back to Healthy.
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let spi = FlakyRunStore::new(5);
    let in_mem: Arc<dyn starter_flow::run::RunStore> = Arc::new(InMemoryRunStore::new());
    let health = HealthHandle::new();

    let runner = FlowRunner::new(store.clone(), in_mem)
        .with_spi_run_store(spi.clone())
        .with_health_handle(health.clone())
        .with_config(long_quiesce_config());

    let node = NodeId::new("com.acme.stage6.identity").unwrap();
    let kind = KindId::new("starter.flow.stage6-identity").unwrap();
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node.clone(),
        Arc::new(Identity { kind }) as Arc<dyn NodeBehavior>,
    );
    let topology = Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        behaviors,
    });
    // Two ticks: the first burns the 5-retry budget (queues the
    // batch + degrades); the second succeeds and drains the queue
    // (= 2 successful store writes), recovering Healthy.
    let seeds = vec![
        (SlotRef::new(node.clone(), "in"), SlotValue::Int(1)),
        (SlotRef::new(node.clone(), "in"), SlotValue::Int(2)),
    ];
    let spec = RunSpec::new(
        FlowId::new("com.acme.stage6.flow").unwrap(),
        FlowRevisionId::new(),
        topology,
        seeds,
        vec![SlotRef::new(node, "out")],
    );

    let mut handle = runner
        .start(spec, SlotMap::new())
        .await
        .expect("start while healthy");
    let _ = timeout(Duration::from_secs(20), &mut handle.join).await;

    // Health is back to Healthy and the store recorded at least
    // two successful checkpoints (the second tick's batch + the
    // drained first tick's batch).
    assert_eq!(health.get(), EngineHealth::Healthy);
    assert!(
        spi.success_count() >= 2,
        "expected >=2 successful checkpoints after recovery, got {}",
        spi.success_count()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lagged_subscriber_increments_subscriber_lagged_count() {
    // Capacity-1 broadcast: pushing 4 events without giving the
    // watcher a chance to consume forces 3 to be evicted; the
    // watcher's next `recv()` returns `Lagged(n)` with n>=1.
    let (tx, _rx) = broadcast::channel::<FlowEvent>(1);
    let metrics = RunMetricsCell::new();
    let watcher = spawn_lagged_watcher(&tx, metrics.clone());

    // Send a flood before yielding, so the watcher task's
    // outstanding recv() resolves to Lagged.
    let run = RunId::new();
    for _ in 0..4 {
        let _ = tx.send(FlowEvent::NodeStarted {
            run,
            node: NodeId::new("com.acme.x").unwrap(),
        });
    }
    // Yield long enough for the watcher to observe the lag.
    for _ in 0..10 {
        sleep(Duration::from_millis(20)).await;
        if metrics.snapshot().subscriber_lagged_count > 0 {
            break;
        }
    }

    let snap = metrics.snapshot();
    assert!(
        snap.subscriber_lagged_count >= 1,
        "expected subscriber_lagged_count >= 1, got {}",
        snap.subscriber_lagged_count
    );

    // Clean up.
    drop(tx);
    let _ = timeout(Duration::from_secs(1), watcher).await;
}
