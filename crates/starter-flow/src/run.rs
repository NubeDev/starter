//! Run lifecycle: `Cancel` plumbing + `RunState` + checkpointing per R6.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" (lifecycle +
//! Cancel propagation) and "Phase 7 — three-level stop" (checkpoint
//! persistence on Pause / Stopped). Owns the per-`RunId` handle the
//! engine hands back to callers and the checkpoint serializer that
//! writes through `RunStore`.
//!
//! ## What lands in stage 7
//!
//! - [`RunCancel`] — already in place from stage 4 (propagator); kept
//!   verbatim. Every `NodeBehavior::invoke` receives a borrow through
//!   `NodeCtx` so node bodies can abort their own work.
//! - [`SkillSelector`] + [`SkillSelection`] — the R7 outer-run binding
//!   seam. [`FlowRunner::start`] calls the selector **exactly once per
//!   outer run**, threads the resulting [`SkillSelection`] through the
//!   run as `Arc<SkillSelection>`, and pins it on the `RunState`. The
//!   `ai-agent` node body itself lands in Phase 4 — wiring the seam
//!   now means Phase 4 does not have to retro-fit it.
//! - [`FlowRunner`] — the per-engine entry point that takes a
//!   [`RunSpec`], asks the [`SkillSelector`] for a [`SkillSelection`],
//!   spawns the stage-4 propagator with the run's [`RunCancel`], and
//!   returns a [`RunHandle`] containing the [`FlowEvent`] broadcast
//!   stream and the run's cancel handle.
//! - [`RunStore`] — the persistence seam. Phase 2 ships the trait + an
//!   in-memory `Vec<RunState>` impl for tests; the SQLite impl lands
//!   in Phase 3 (per the flow SCOPE Phase 3 block).
//!
//! ## What does NOT land here yet
//!
//! - Three-level stop (Pause / Stop with checkpoint flush) lands in
//!   Phase 7 against a richer `RunStore` API.
//! - `Pause` / `Resume` per-run plumbing is the engine's concern (stage
//!   6 [`crate::engine`]); this stage stops a run by cancelling its
//!   [`RunCancel`] handle.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{broadcast, Notify, RwLock};
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};

use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId, RunId};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Cancel;

use crate::propagator::{self, FlowTopology, PropagatorConfig};
use crate::state::{RunState, RunStatus};

/// Per-run cancellation handle.
///
/// SCOPE R13 — cancellation across the flow engine reuses the existing
/// [`Cancel`] seam. The propagator awaits [`Cancel::cancelled`] in its
/// main `select!` so a fired cancel stops scheduling further hops
/// within a few hundred milliseconds; every `NodeBehavior::invoke`
/// receives a borrow of this handle through `NodeCtx` so node bodies
/// can abort their own work too.
///
/// The implementation is a plain `AtomicBool` plus a [`Notify`] — no
/// `tokio_util` dependency leaks into this crate. Construction goes
/// through [`Self::new`] which returns an [`Arc`] because the engine
/// hands the same handle to (a) the propagator task, (b) every
/// in-flight node invocation, and (c) the public run handle the
/// engine's caller can flip.
#[derive(Debug, Default)]
pub struct RunCancel {
    flag: AtomicBool,
    notify: Notify,
}

impl RunCancel {
    /// Construct a fresh, un-cancelled run cancel handle.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Flip the cancel flag and wake every waiter. Idempotent — a
    /// second call is a no-op.
    pub fn cancel(&self) {
        if !self.flag.swap(true, Ordering::SeqCst) {
            self.notify.notify_waiters();
        }
    }
}

impl Cancel for RunCancel {
    fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            if self.flag.load(Ordering::SeqCst) {
                return;
            }
            loop {
                // Register the waiter *before* re-checking the flag so
                // we don't miss a `notify_waiters()` racing with our
                // load.
                let notified = self.notify.notified();
                if self.flag.load(Ordering::SeqCst) {
                    return;
                }
                notified.await;
                if self.flag.load(Ordering::SeqCst) {
                    return;
                }
            }
        })
    }
}

/// Skill selection produced by a [`SkillSelector`] at the top of an
/// outer flow run.
///
/// SCOPE: the canonical `SkillSelection` shape lives in
/// `starter-skills`, which is not yet a workspace member. This Phase-2
/// placeholder pins the *seam* — every `RunState` carries an
/// `Option<Arc<SkillSelection>>` and the `ai-agent` node body (Phase 4)
/// will read it via the `RunState`. When `starter-skills` lands, drop
/// this placeholder and re-export the canonical type from
/// [`starter_flow_spi::skill`].
///
/// `#[non_exhaustive]` so adding fields when the real type lands is
/// not a breaking change for code that constructs the placeholder.
#[non_exhaustive]
#[derive(Debug, Default, Clone)]
pub struct SkillSelection {
    /// Free-form selection label. Phase 4 / `starter-skills` replaces
    /// this with the canonical structured selection.
    pub label: String,
}

impl SkillSelection {
    /// Construct a [`SkillSelection`] with a free-form label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Outer-run skill selection hook (R7).
///
/// SCOPE R7: the AI agent is a *node kind*, not a runtime — and skill
/// selection runs **once per outer flow run**, with the result threaded
/// through every `ai-agent` node in that run. This trait is the seam
/// [`FlowRunner::start`] calls exactly once at the top of every run.
///
/// Phase 2 wires the seam; Phase 4 fleshes out the `ai-agent` body
/// that reads the selection.
#[async_trait]
pub trait SkillSelector: Send + Sync + 'static {
    /// Select the skills for one outer run.
    async fn select(
        &self,
        flow: &FlowId,
        revision: &FlowRevisionId,
        input: &SlotMap,
    ) -> SkillSelection;
}

/// Trivial selector that always returns an empty [`SkillSelection`].
///
/// Default plug for engines that have not been configured with a real
/// selector — keeps the R7 seam alive on every run without imposing a
/// real skills dependency in Phase 2.
pub struct NoopSkillSelector;

#[async_trait]
impl SkillSelector for NoopSkillSelector {
    async fn select(
        &self,
        _flow: &FlowId,
        _revision: &FlowRevisionId,
        _input: &SlotMap,
    ) -> SkillSelection {
        SkillSelection::default()
    }
}

/// Persistence seam for flow runs and per-run checkpoints (R6).
///
/// SCOPE: Phase 2 is a **trait seam only**. The in-memory impl
/// [`InMemoryRunStore`] is a `Vec<Arc<RwLock<RunState>>>` for tests;
/// the SQLite impl lands in Phase 3 (per the flow SCOPE Phase 3
/// block).
///
/// The trait deliberately takes `Arc<RwLock<RunState>>` rather than
/// `RunState` so the runner can keep mutating the same record after
/// `record` returns — there is no separate "save" call in Phase 2.
#[async_trait]
pub trait RunStore: Send + Sync + 'static {
    /// Record a freshly-created [`RunState`]. The store retains a
    /// shared handle and may observe later mutations.
    async fn record(&self, state: Arc<RwLock<RunState>>);

    /// Fetch the recorded state for a [`RunId`], or `None` if unknown.
    async fn get(&self, run: RunId) -> Option<Arc<RwLock<RunState>>>;

    /// Snapshot count (test / inspector convenience).
    async fn len(&self) -> usize;

    /// Whether the store has zero recorded runs.
    async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
}

/// In-memory [`RunStore`] backed by a `Vec` of run records.
///
/// Phase 2 only — the SQLite impl lands in Phase 3 alongside the rest
/// of `starter-store-sqlite`.
#[derive(Default)]
pub struct InMemoryRunStore {
    inner: RwLock<Vec<Arc<RwLock<RunState>>>>,
}

impl InMemoryRunStore {
    /// Construct an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RunStore for InMemoryRunStore {
    async fn record(&self, state: Arc<RwLock<RunState>>) {
        self.inner.write().await.push(state);
    }

    async fn get(&self, run: RunId) -> Option<Arc<RwLock<RunState>>> {
        let inner = self.inner.read().await;
        for r in inner.iter() {
            if r.read().await.run == run {
                return Some(r.clone());
            }
        }
        None
    }

    async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
}

/// Caller-supplied description of a single flow run.
///
/// Phase 2 ships the minimum the propagator needs: a [`FlowTopology`]
/// (the resolved nodes/links/triggers/behaviors map from stage 4), the
/// `(flow, revision)` pair that pins the run to an immutable
/// revision, the seeded slot writes that kick the propagator, and the
/// list of terminal output slots the runner reads back for
/// `FlowEvent::RunCompleted`.
///
/// In Phase 3 the runner will derive the topology from
/// [`crate::registry::FlowRegistry`] + [`crate::registry::NodeKindRegistry`]
/// internally; for Phase 2 the caller assembles it.
#[non_exhaustive]
pub struct RunSpec {
    /// The flow this run executes.
    pub flow: FlowId,
    /// The immutable revision of the flow this run is bound to.
    pub revision: FlowRevisionId,
    /// Resolved topology (links, triggers, behaviors).
    pub topology: Arc<FlowTopology>,
    /// Seed writes performed *after* `RunStarted` is emitted and the
    /// propagator is subscribed. The first seed kicks the chain.
    pub seeds: Vec<(SlotRef, SlotValue)>,
    /// Slots whose values are gathered into the `RunCompleted.output`
    /// map at the end of a successful run.
    pub terminal_slots: Vec<SlotRef>,
}

/// Tunable knobs on [`FlowRunner::start`].
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FlowRunnerConfig {
    /// Propagator policies (R1 cycle-bound budget).
    pub propagator: PropagatorConfig,
    /// Quiescence window: after this much time elapses with no
    /// propagator events, the runner treats the run as complete and
    /// emits `FlowEvent::RunCompleted`. Default 100 ms — long enough
    /// for an in-memory propagator hop to schedule a follow-up, short
    /// enough that tests need not wait seconds for completion.
    pub quiescence: Duration,
    /// Capacity of the run's [`FlowEvent`] broadcast channel.
    pub event_buffer: usize,
}

impl Default for FlowRunnerConfig {
    fn default() -> Self {
        Self {
            propagator: PropagatorConfig::default(),
            quiescence: Duration::from_millis(100),
            event_buffer: 256,
        }
    }
}

/// Handle returned by [`FlowRunner::start`].
///
/// The caller subscribes to [`Self::events_tx`] for the run's
/// `FlowEvent` stream, can flip [`Self::cancel`] to abort the run, and
/// awaits [`Self::join`] to observe the terminal [`RunStatus`].
///
/// [`Self::initial_rx`] is a pre-subscribed receiver the runner sets
/// up *before* spawning the coordinator task; using it guarantees the
/// caller never misses the leading `FlowEvent::RunStarted`.
#[non_exhaustive]
pub struct RunHandle {
    /// The run id.
    pub run: RunId,
    /// Per-run cancel handle.
    pub cancel: Arc<RunCancel>,
    /// `FlowEvent` broadcast sender. Multi-consumer; call
    /// [`broadcast::Sender::subscribe`] for additional receivers.
    pub events_tx: broadcast::Sender<FlowEvent>,
    /// Pre-subscribed receiver. The runner subscribes synchronously
    /// *before* spawning the coordinator so this receiver is
    /// guaranteed to see `RunStarted`.
    pub initial_rx: broadcast::Receiver<FlowEvent>,
    /// Coordinator task handle. Resolves to the terminal [`RunStatus`]
    /// once the run finishes.
    pub join: JoinHandle<RunStatus>,
}

/// The per-engine entry point that turns a [`RunSpec`] into a live
/// run.
///
/// Owns the [`GraphStore`], the [`RunStore`] seam, and an optional
/// [`SkillSelector`] hook. Phase 3 will additionally take
/// [`crate::registry::FlowRegistry`] + [`crate::registry::NodeKindRegistry`]
/// references so callers can submit a `(FlowId, FlowRevisionId)` pair
/// and let the runner derive the topology.
pub struct FlowRunner {
    store: Arc<dyn GraphStore>,
    run_store: Arc<dyn RunStore>,
    skill_selector: Arc<dyn SkillSelector>,
    config: FlowRunnerConfig,
}

impl FlowRunner {
    /// Construct a [`FlowRunner`] with default config.
    pub fn new(store: Arc<dyn GraphStore>, run_store: Arc<dyn RunStore>) -> Self {
        Self {
            store,
            run_store,
            skill_selector: Arc::new(NoopSkillSelector),
            config: FlowRunnerConfig::default(),
        }
    }

    /// Replace the [`SkillSelector`] hook.
    pub fn with_skill_selector(mut self, selector: Arc<dyn SkillSelector>) -> Self {
        self.skill_selector = selector;
        self
    }

    /// Replace the runner config.
    pub fn with_config(mut self, config: FlowRunnerConfig) -> Self {
        self.config = config;
        self
    }

    /// Start a run.
    ///
    /// SCOPE R7 outer-run binding rule: the [`SkillSelector`] hook is
    /// called **exactly once per `FlowRunner::start`** — before the
    /// propagator is spawned and before any seed write hits the store.
    /// The resulting [`SkillSelection`] is pinned on the `RunState` as
    /// `Arc<SkillSelection>` and made available to every `ai-agent`
    /// node body via the `RunState` (Phase 4 reads it).
    ///
    /// The returned [`RunHandle::initial_rx`] is subscribed
    /// synchronously *before* the coordinator task spawns, so the
    /// caller is guaranteed not to miss `FlowEvent::RunStarted`.
    pub async fn start(&self, spec: RunSpec, input: SlotMap) -> RunHandle {
        // 1. Skill selection — exactly once per outer run (R7).
        let selection = Arc::new(
            self.skill_selector
                .select(&spec.flow, &spec.revision, &input)
                .await,
        );

        // 2. Per-run primitives.
        let run = RunId::new();
        let cancel = RunCancel::new();
        let (events_tx, _) = broadcast::channel::<FlowEvent>(self.config.event_buffer);

        // 3. Record the RunState with the in-memory RunStore. The
        //    coordinator below mutates this same Arc<RwLock<_>>.
        let state = Arc::new(RwLock::new(RunState::new(
            run,
            spec.flow.clone(),
            spec.revision,
            events_tx.clone(),
            cancel.clone(),
            Some(selection.clone()),
        )));
        self.run_store.record(state.clone()).await;

        // 4. Subscribe BEFORE spawning so the caller never misses
        //    `RunStarted` (the broadcast channel only delivers to
        //    receivers that exist at send time).
        let initial_rx = events_tx.subscribe();
        let coordinator_rx = events_tx.subscribe();

        let store = self.store.clone();
        let cfg = self.config;
        let RunSpec {
            flow,
            revision: _,
            topology,
            seeds,
            terminal_slots,
        } = spec;

        let cancel_for_task = cancel.clone();
        let events_for_task = events_tx.clone();
        let state_for_task = state.clone();

        let join = tokio::spawn(async move {
            run_coordinator(
                run,
                flow,
                store,
                topology,
                cancel_for_task,
                events_for_task,
                coordinator_rx,
                seeds,
                terminal_slots,
                state_for_task,
                cfg,
            )
            .await
        });

        RunHandle {
            run,
            cancel,
            events_tx,
            initial_rx,
            join,
        }
    }
}

/// Coordinator task body. Owns the per-run choreography:
///
/// 1. Mark `RunState::Running`, emit `RunStarted`.
/// 2. Spawn the stage-4 propagator with the run's `RunCancel` and
///    `events_tx`.
/// 3. Drive the seed writes through the single `GraphStore::write_slot`
///    chokepoint (R2). The first seed kicks the propagator.
/// 4. Watch the event stream: forward propagator-emitted terminal
///    events (`RunCancelled`, `RunFailed`); on quiescence (no events
///    for `cfg.quiescence`), gather the terminal-slot output and
///    emit `RunCompleted`.
/// 5. Cancel the propagator on the way out so its task drains, then
///    finalise `RunState::status` and return it.
#[allow(clippy::too_many_arguments)]
async fn run_coordinator(
    run: RunId,
    flow: FlowId,
    store: Arc<dyn GraphStore>,
    topology: Arc<FlowTopology>,
    cancel: Arc<RunCancel>,
    events_tx: broadcast::Sender<FlowEvent>,
    mut events_rx: broadcast::Receiver<FlowEvent>,
    seeds: Vec<(SlotRef, SlotValue)>,
    terminal_slots: Vec<SlotRef>,
    state: Arc<RwLock<RunState>>,
    cfg: FlowRunnerConfig,
) -> RunStatus {
    // Mark Running.
    {
        let mut st = state.write().await;
        st.status = RunStatus::Running;
    }

    // RunStarted goes out before anything else.
    let _ = events_tx.send(FlowEvent::RunStarted {
        run,
        flow: flow.clone(),
    });

    // Spawn propagator.
    let prop_handle = propagator::spawn(
        store.clone(),
        topology,
        cancel.clone(),
        events_tx.clone(),
        run,
        cfg.propagator,
    );

    // Seed writes — these enter through the single chokepoint per R2.
    for (slot, value) in seeds {
        if cancel.is_cancelled() {
            break;
        }
        if let Err(e) = store.write_slot(&slot, value, WriteSlotOpts::live()).await {
            tracing::warn!(
                run = %run,
                target = ?slot,
                error = %e,
                "run coordinator seed write failed",
            );
        }
    }

    // Quiescence loop. After every event we reset the deadline to
    // `now + cfg.quiescence`. If the deadline elapses with no fresh
    // event, the run is treated as complete.
    let mut terminal: Option<RunStatus> = None;
    let mut deadline = Instant::now() + cfg.quiescence;

    while terminal.is_none() {
        tokio::select! {
            biased;
            recv = events_rx.recv() => {
                match recv {
                    Ok(ev) => {
                        match ev {
                            FlowEvent::RunCancelled { .. } => {
                                terminal = Some(RunStatus::Cancelled);
                            }
                            FlowEvent::RunFailed { error, .. } => {
                                terminal = Some(RunStatus::Failed(error));
                            }
                            FlowEvent::RunCompleted { .. } => {
                                terminal = Some(RunStatus::Completed);
                            }
                            _ => {
                                deadline = Instant::now() + cfg.quiescence;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        deadline = Instant::now() + cfg.quiescence;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            _ = sleep_until(deadline) => {
                // Quiescence: read terminal output and emit RunCompleted.
                let mut output = SlotMap::new();
                for sr in &terminal_slots {
                    if let Ok(v) = store.read_slot(sr).await {
                        output.insert(format!("{}.{}", sr.node, sr.slot), v);
                    }
                }
                let _ = events_tx.send(FlowEvent::RunCompleted { run, output });
                terminal = Some(RunStatus::Completed);
            }
        }
    }

    // Tear down the propagator. `cancel` fires it; we then await the
    // join handle so the task exits cleanly. The propagator's own
    // RunCancelled emission on shutdown is fine — we're already
    // terminal so subscribers see at most a duplicate trailing event.
    cancel.cancel();
    let _ = tokio::time::timeout(Duration::from_secs(1), prop_handle).await;

    // Persist terminal status on RunState.
    let final_status = terminal.unwrap_or(RunStatus::Cancelled);
    {
        let mut st = state.write().await;
        st.status = final_status.clone();
    }
    final_status
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::InMemoryGraphStore;

    use std::sync::atomic::AtomicU64;
    use std::time::Duration;

    use async_trait::async_trait;
    use starter_flow_spi::node::{
        KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotValue,
    };
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use tokio::time::{sleep, timeout};

    fn nid(s: &str) -> NodeId {
        NodeId::new(s).unwrap()
    }
    fn slot(node: &str, name: &str) -> SlotRef {
        SlotRef::new(nid(node), name)
    }

    struct Identity {
        kind: KindId,
        calls: Arc<AtomicU64>,
    }
    impl Identity {
        fn new() -> (Arc<Self>, Arc<AtomicU64>) {
            let calls = Arc::new(AtomicU64::new(0));
            let kind = KindId::new("starter.flow.run-test-identity").unwrap();
            (
                Arc::new(Self {
                    kind,
                    calls: calls.clone(),
                }),
                calls,
            )
        }
    }
    #[async_trait]
    impl NodeBehavior for Identity {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let v = input.get("in").cloned().unwrap_or(SlotValue::Null);
            let mut out = SlotMap::new();
            out.insert("out".to_owned(), v);
            Ok(out)
        }
    }

    struct Incrementer {
        kind: KindId,
    }
    impl Incrementer {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new("starter.flow.run-test-incrementer").unwrap(),
            })
        }
    }
    #[async_trait]
    impl NodeBehavior for Incrementer {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
            let n = match input.get("in") {
                Some(SlotValue::Int(n)) => *n,
                _ => 0,
            };
            let mut out = SlotMap::new();
            out.insert("out".to_owned(), SlotValue::Int(n + 1));
            Ok(out)
        }
    }

    /// Build an A → B → C identity chain.
    fn identity_chain_topology() -> (Arc<FlowTopology>, Arc<AtomicU64>, Arc<AtomicU64>) {
        let (b, b_calls) = Identity::new();
        let (c, c_calls) = Identity::new();
        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.a", "out"), vec![slot("flow.test.b", "in")]);
        links.insert(slot("flow.test.b", "out"), vec![slot("flow.test.c", "in")]);
        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.b"), BTreeSet::from(["in".to_owned()]));
        triggers.insert(nid("flow.test.c"), BTreeSet::from(["in".to_owned()]));
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.b"), b);
        behaviors.insert(nid("flow.test.c"), c);
        (
            Arc::new(FlowTopology {
                links,
                triggers,
                behaviors,
            }),
            b_calls,
            c_calls,
        )
    }

    async fn drain(rx: &mut broadcast::Receiver<FlowEvent>, dur: Duration) -> Vec<FlowEvent> {
        let mut out = Vec::new();
        loop {
            match timeout(dur, rx.recv()).await {
                Ok(Ok(ev)) => out.push(ev),
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => break,
                Err(_) => break,
            }
        }
        out
    }

    // ---------- RunCancel parity tests (carried over from stage 4) ----------

    #[tokio::test]
    async fn cancel_flips_and_wakes_waiters() {
        let c = RunCancel::new();
        assert!(!c.is_cancelled());
        let c2 = c.clone();
        let waiter = tokio::spawn(async move {
            c2.cancelled().await;
        });
        sleep(Duration::from_millis(20)).await;
        c.cancel();
        timeout(Duration::from_millis(200), waiter)
            .await
            .expect("waiter never woke")
            .expect("waiter task panicked");
        assert!(c.is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_returns_immediately_if_already_cancelled() {
        let c = RunCancel::new();
        c.cancel();
        timeout(Duration::from_millis(50), c.cancelled())
            .await
            .expect("cancelled() did not resolve immediately for an already-cancelled token");
    }

    // ---------- Stage 7 lifecycle tests ----------

    /// Successful run: RunStarted → NodeStarted+ → NodeEmitted+ →
    /// RunCompleted. A two-node identity chain so we observe at least
    /// one `NodeStarted` + `NodeEmitted` pair per node.
    #[tokio::test]
    async fn successful_run_emits_started_then_node_events_then_completed() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let runner = FlowRunner::new(store.clone(), run_store.clone());

        let (topology, b_calls, c_calls) = identity_chain_topology();
        let spec = RunSpec {
            flow: FlowId::new("flow.test.success").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.a", "out"), SlotValue::Int(42))],
            terminal_slots: vec![slot("flow.test.c", "out")],
        };

        let mut handle = runner.start(spec, SlotMap::new()).await;
        let status = timeout(Duration::from_secs(2), &mut handle.join)
            .await
            .expect("coordinator did not exit in time")
            .expect("coordinator panicked");

        assert_eq!(status, RunStatus::Completed);
        assert_eq!(b_calls.load(Ordering::SeqCst), 1);
        assert_eq!(c_calls.load(Ordering::SeqCst), 1);

        let events = drain(&mut handle.initial_rx, Duration::from_millis(50)).await;

        // Required prefix: starts with RunStarted.
        assert!(
            matches!(events.first(), Some(FlowEvent::RunStarted { .. })),
            "first event must be RunStarted; got {events:?}",
        );

        let node_started = events
            .iter()
            .filter(|e| matches!(e, FlowEvent::NodeStarted { .. }))
            .count();
        let node_emitted = events
            .iter()
            .filter(|e| matches!(e, FlowEvent::NodeEmitted { .. }))
            .count();
        assert!(node_started >= 1, "expected ≥1 NodeStarted; got {events:?}");
        assert!(node_emitted >= 1, "expected ≥1 NodeEmitted; got {events:?}");

        let completed = events
            .iter()
            .find(|e| matches!(e, FlowEvent::RunCompleted { .. }));
        let Some(FlowEvent::RunCompleted { output, .. }) = completed else {
            panic!("expected RunCompleted in events; got {events:?}");
        };
        let key = "flow.test.c.out";
        assert_eq!(
            output.get(key),
            Some(&SlotValue::Int(42)),
            "RunCompleted.output must include terminal slot; got {output:?}",
        );

        // RunState was recorded and reflects the terminal status.
        assert_eq!(run_store.len().await, 1);
        let recorded = run_store.get(handle.run).await.expect("run was recorded");
        assert_eq!(recorded.read().await.status, RunStatus::Completed);
    }

    /// Cancelled run: cancel mid-flight, expect `RunCancelled` within
    /// bounded time.
    #[tokio::test]
    async fn cancelled_run_emits_run_cancelled_within_bounded_time() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let runner =
            FlowRunner::new(store.clone(), run_store.clone()).with_config(FlowRunnerConfig {
                // Long quiescence so the run does not finish via the
                // completion path before our cancel arrives. Generous
                // hop budget so the propagator does not exhaust it
                // before our cancel arrives either.
                quiescence: Duration::from_secs(10),
                propagator: PropagatorConfig {
                    max_propagation_hops: 1_000_000,
                },
                ..FlowRunnerConfig::default()
            });

        // Incrementer self-loop so the propagator keeps working.
        let inc = Incrementer::new();
        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.n", "out"), vec![slot("flow.test.n", "in")]);
        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.n"), BTreeSet::from(["in".to_owned()]));
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.n"), inc);
        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let spec = RunSpec {
            flow: FlowId::new("flow.test.cancel").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.n", "in"), SlotValue::Int(1))],
            terminal_slots: vec![],
        };

        let mut handle = runner.start(spec, SlotMap::new()).await;

        // Give the propagator a moment to start, then cancel.
        sleep(Duration::from_millis(50)).await;
        handle.cancel.cancel();

        let status = timeout(Duration::from_secs(1), &mut handle.join)
            .await
            .expect("coordinator did not exit within 1s of cancel")
            .expect("coordinator panicked");
        assert_eq!(status, RunStatus::Cancelled);

        let events = drain(&mut handle.initial_rx, Duration::from_millis(50)).await;
        assert!(
            events
                .iter()
                .any(|e| matches!(e, FlowEvent::RunCancelled { .. })),
            "expected RunCancelled; got {events:?}",
        );
    }

    /// Cycle-exhausted run: forced-no-shortcut incrementer self-loop
    /// with a tiny budget; expect `RunFailed` with reason mentioning
    /// the cycle budget.
    #[tokio::test]
    async fn cycle_exhausted_run_emits_run_failed_with_cycle_budget() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let runner =
            FlowRunner::new(store.clone(), run_store.clone()).with_config(FlowRunnerConfig {
                propagator: PropagatorConfig {
                    max_propagation_hops: 5,
                },
                quiescence: Duration::from_millis(500),
                ..FlowRunnerConfig::default()
            });

        let inc = Incrementer::new();
        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(slot("flow.test.n", "out"), vec![slot("flow.test.n", "in")]);
        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.n"), BTreeSet::from(["in".to_owned()]));
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.n"), inc);
        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        let spec = RunSpec {
            flow: FlowId::new("flow.test.cycle").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.n", "in"), SlotValue::Int(1))],
            terminal_slots: vec![],
        };

        let mut handle = runner.start(spec, SlotMap::new()).await;
        let status = timeout(Duration::from_secs(2), &mut handle.join)
            .await
            .expect("coordinator did not exit on cycle exhaustion")
            .expect("coordinator panicked");

        let RunStatus::Failed(ref reason) = status else {
            panic!("expected Failed; got {status:?}");
        };
        assert!(
            reason.contains("cycle budget"),
            "Failed reason must mention cycle budget; got {reason:?}",
        );

        let events = drain(&mut handle.initial_rx, Duration::from_millis(50)).await;
        let found = events.iter().any(|e| {
            matches!(
                e,
                FlowEvent::RunFailed { error, .. } if error.contains("cycle budget")
            )
        });
        assert!(found, "expected RunFailed(cycle-budget); got {events:?}");
    }

    /// SCOPE R7 outer-run binding: the [`SkillSelector`] hook is
    /// called **exactly once per `FlowRunner::start`** — regardless
    /// of how many `ai-agent` nodes the run contains (Phase 4 will
    /// validate the read side).
    #[tokio::test]
    async fn skill_selector_called_exactly_once_per_start() {
        struct Counting {
            calls: Arc<AtomicU64>,
        }
        #[async_trait]
        impl SkillSelector for Counting {
            async fn select(
                &self,
                _flow: &FlowId,
                _revision: &FlowRevisionId,
                _input: &SlotMap,
            ) -> SkillSelection {
                self.calls.fetch_add(1, Ordering::SeqCst);
                SkillSelection::new("counted")
            }
        }

        let calls = Arc::new(AtomicU64::new(0));
        let selector = Arc::new(Counting {
            calls: calls.clone(),
        });

        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let runner = FlowRunner::new(store, run_store.clone()).with_skill_selector(selector);

        let (topology, _, _) = identity_chain_topology();
        let spec = RunSpec {
            flow: FlowId::new("flow.test.skill").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.a", "out"), SlotValue::Int(1))],
            terminal_slots: vec![slot("flow.test.c", "out")],
        };

        let mut handle = runner.start(spec, SlotMap::new()).await;
        let _ = timeout(Duration::from_secs(2), &mut handle.join).await;

        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "SkillSelector::select must be called exactly once per FlowRunner::start",
        );

        // And the selection threaded onto the RunState is the one we
        // produced.
        let recorded = run_store.get(handle.run).await.expect("run recorded");
        let st = recorded.read().await;
        let sel = st
            .skill_selection
            .as_ref()
            .expect("skill selection must be pinned on RunState");
        assert_eq!(sel.label, "counted");
    }
}
