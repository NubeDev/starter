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

use starter_flow_spi::flow::{
    DedupKey, EngineError, FlowEvent, FlowId, FlowRevisionId, RunCheckpoint, RunId, RunOpts,
    RunOutcome, RunStore as SpiRunStore,
};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{SlotMap, SlotRef, SlotValue};
use starter_flow_spi::{Cancel, Principal};

use crate::health::HealthHandle;
use crate::metrics::{spawn_lagged_watcher, RunMetricsCell};
use crate::propagator::{self, CheckpointHook, DegradedQueue, FlowTopology, PropagatorConfig};
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

/// Re-export of the SPI [`SkillSelection`](starter_flow_spi::skill::SkillSelection).
///
/// Phase 4 stage 3 promoted the placeholder local type to the real
/// `starter-flow-spi::skill::SkillSelection` (a `Selected { skill_id,
/// allowed_tools, resources, content_hash } | None` enum); this
/// re-export keeps existing callsites in `starter-flow::run` /
/// `starter-flow::state` working without renaming the import.
pub use starter_flow_spi::skill::SkillSelection;

/// Re-export of the SPI [`SkillSelector`](starter_flow_spi::skill::SkillSelector).
pub use starter_flow_spi::skill::SkillSelector;

/// Re-export of the SPI [`NullSkillSelector`](starter_flow_spi::skill::NullSkillSelector)
/// — the engine default when no selector is registered. Returns
/// [`SkillSelection::None`] for every run.
pub use starter_flow_spi::skill::NullSkillSelector;

/// Re-export of the SPI [`SkillError`](starter_flow_spi::skill::SkillError).
pub use starter_flow_spi::skill::SkillError;

/// `system/Admin` `Principal` used when neither `RunSpec::with_principal`
/// nor `NodeCtx` carries a real identity (Phase 2 stage-5 default;
/// retired once `NodeCtx` grows a `Principal` field in a later
/// phase).
fn default_system_admin_principal() -> Principal {
    Principal {
        subject: "system/Admin".to_string(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
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
    /// Optional caller-supplied [`Principal`] threaded into
    /// `SpiRunStore::start`. `None` falls back to the engine's
    /// `system/Admin` default. Set via [`Self::with_principal`].
    /// Lands here in stage 8 so `FlowAsService` (and surfaces in
    /// general) can record the on-behalf-of identity that drove a
    /// per-event run.
    pub principal: Option<Principal>,
    /// Optional caller-supplied [`DedupKey`] threaded into
    /// `SpiRunStore::start` so future `find_by_dedup_key` lookups
    /// can short-circuit re-deliveries (D-F3.12). Set via
    /// [`Self::with_dedup_key`].
    pub dedup_key: Option<DedupKey>,
}

impl RunSpec {
    /// Construct a [`RunSpec`]. Public constructor for downstream
    /// crates that cannot use the struct-expression form because
    /// of `#[non_exhaustive]`.
    pub fn new(
        flow: FlowId,
        revision: FlowRevisionId,
        topology: Arc<FlowTopology>,
        seeds: Vec<(SlotRef, SlotValue)>,
        terminal_slots: Vec<SlotRef>,
    ) -> Self {
        Self {
            flow,
            revision,
            topology,
            seeds,
            terminal_slots,
            principal: None,
            dedup_key: None,
        }
    }

    /// Attach a [`Principal`] to be recorded on the run row by the
    /// SPI `RunStore::start` call. Retires the stage-5
    /// `system/Admin` hardcode for surfaces that have a real
    /// identity to thread through (D-F3.12 + R7 outer-run
    /// binding).
    pub fn with_principal(mut self, principal: Principal) -> Self {
        self.principal = Some(principal);
        self
    }

    /// Attach a [`DedupKey`] for D-F3.12 short-circuit lookups.
    /// `FlowAsService` resolves the key per-event and threads it
    /// here so the SPI `RunStore` can persist it under the
    /// `UNIQUE (service_name, dedup_key)` partial index.
    pub fn with_dedup_key(mut self, dedup: DedupKey) -> Self {
        self.dedup_key = Some(dedup);
        self
    }
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
    /// Per-run live metrics counters (D-F3.10 + D-F3.11). The
    /// runner threads the same `Arc` into the propagator's
    /// [`CheckpointHook`] and into the engine-owned `Lagged`-watcher
    /// subscriber, so reads here observe live increments.
    pub metrics: Arc<RunMetricsCell>,
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
    /// Phase 3 SPI [`SpiRunStore`] for run lifecycle + per-tick
    /// checkpoint persistence (R6, D-F3.2). `None` means the runner
    /// is in Phase-2 in-memory-only mode and the propagator runs
    /// without a checkpoint hook (`Engine` behaves exactly as it
    /// does today).
    spi_run_store: Option<Arc<dyn SpiRunStore>>,
    /// Per-run knobs (D-F3.9 retention, D-F3.10 broadcast capacity,
    /// D-F3.11 degraded queue capacity). Defaulted from
    /// [`RunOpts::default`]; per-run override lands in stage 6.
    run_opts: RunOpts,
    /// Engine-level health flag (D-F3.11). Shared with the engine
    /// via [`Self::with_health_handle`]; the propagator's per-tick
    /// retry-with-backoff loop flips this on degrade / recovery and
    /// [`Self::start`] reads it to reject new runs while
    /// [`starter_flow_spi::flow::EngineHealth::Degraded`].
    health: HealthHandle,
    skill_selector: Arc<dyn SkillSelector>,
    config: FlowRunnerConfig,
    /// Per-node persistent state seam threaded into every
    /// `NodeCtx.state`. Defaults to
    /// [`starter_flow_spi::state::NoopNodeStateStore`]; hosts that
    /// need durable per-instance state (counter node, etc.) swap in
    /// a real backend via [`Self::with_node_state_store`].
    node_state_store: Arc<dyn starter_flow_spi::state::NodeStateStore>,
}

impl FlowRunner {
    /// Construct a [`FlowRunner`] with default config.
    pub fn new(store: Arc<dyn GraphStore>, run_store: Arc<dyn RunStore>) -> Self {
        Self {
            store,
            run_store,
            spi_run_store: None,
            run_opts: RunOpts::default(),
            health: HealthHandle::new(),
            skill_selector: Arc::new(NullSkillSelector),
            config: FlowRunnerConfig::default(),
            node_state_store: Arc::new(starter_flow_spi::state::NoopNodeStateStore),
        }
    }

    /// Swap the per-run [`NodeStateStore`](starter_flow_spi::state::NodeStateStore)
    /// the propagator threads into every `NodeCtx`. Defaults to
    /// [`starter_flow_spi::state::NoopNodeStateStore`]; production
    /// surfaces pass `Arc<SqliteNodeStateStore>` here so counter
    /// nodes (and any other stateful kind) persist across runs.
    pub fn with_node_state_store(
        mut self,
        store: Arc<dyn starter_flow_spi::state::NodeStateStore>,
    ) -> Self {
        self.node_state_store = store;
        self
    }

    /// Attach a Phase 3 SPI [`SpiRunStore`] (R6, D-F3.2). When
    /// attached, every run started by this runner persists its
    /// lifecycle (`start` / `finish`) and per-tick checkpoint
    /// batches through the SPI store; the resume path
    /// ([`Self::resume`]) reads back the latest checkpoint and
    /// replays its slot writes through the single
    /// [`GraphStore::write_slot`] chokepoint (R2 unchanged).
    pub fn with_spi_run_store(mut self, store: Arc<dyn SpiRunStore>) -> Self {
        self.spi_run_store = Some(store);
        self
    }

    /// Replace the per-run [`RunOpts`].
    pub fn with_run_opts(mut self, opts: RunOpts) -> Self {
        self.run_opts = opts;
        self
    }

    /// Share the engine's health flag (D-F3.11). When the engine
    /// degrades, [`Self::start`] returns
    /// [`EngineError::BackendUnavailable`]; when the next
    /// `RunStore::checkpoint` succeeds the engine recovers and
    /// new `start(...)` calls are accepted again.
    pub fn with_health_handle(mut self, health: HealthHandle) -> Self {
        self.health = health;
        self
    }

    /// Borrow the current health handle (test convenience).
    pub fn health_handle(&self) -> &HealthHandle {
        &self.health
    }

    /// Borrow the attached [`SpiRunStore`] if any.
    pub fn spi_run_store(&self) -> Option<&Arc<dyn SpiRunStore>> {
        self.spi_run_store.as_ref()
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
    ///
    /// Stage 6 (D-F3.11): rejects new runs with
    /// [`EngineError::BackendUnavailable`] while the engine is in
    /// [`starter_flow_spi::flow::EngineHealth::Degraded`]. The
    /// rejection is fast (a single `AtomicU8` load) and happens
    /// before any per-run allocation, so the runner sheds load
    /// cleanly when the backend is unreachable.
    pub async fn start(&self, spec: RunSpec, input: SlotMap) -> Result<RunHandle, EngineError> {
        use starter_flow_spi::flow::EngineHealth;
        if self.health.get() == EngineHealth::Degraded {
            return Err(EngineError::BackendUnavailable);
        }
        Ok(self.launch(spec, input, None).await)
    }

    /// Resume an in-flight run whose state was persisted via a
    /// Phase 3 SPI [`SpiRunStore`] before the previous process
    /// exited.
    ///
    /// Returns `Ok(Some(_))` if the store had a checkpoint for
    /// `run_id` (the slot writes are replayed through the single
    /// [`GraphStore::write_slot`] chokepoint per R2 and a fresh
    /// propagator picks up from `initial_seq = checkpoint.seq`);
    /// `Ok(None)` if no checkpoint exists; `Err` if no SPI store
    /// is attached.
    ///
    /// SCOPE Phase 3 "Run checkpointing wired into the engine":
    /// the resume path is **not** a second writer — it goes through
    /// the same `GraphStore::write_slot`, and the propagator's
    /// short-circuit on idempotent writes (D1a) absorbs the no-op
    /// writes that already-current slots produce. The freshly-
    /// spawned propagator's `initial_seq` is set so its first
    /// post-resume checkpoint carries `seq = checkpoint.seq + 1`,
    /// keeping `(run_id, seq)` strictly monotonic across the
    /// SIGKILL boundary.
    pub async fn resume(
        &self,
        spec: RunSpec,
        input: SlotMap,
        run_id: RunId,
    ) -> Result<Option<RunHandle>, FlowRunnerError> {
        let Some(spi) = self.spi_run_store.clone() else {
            return Err(FlowRunnerError::NoRunStore);
        };
        let checkpoint = spi
            .load(run_id)
            .await
            .map_err(|e| FlowRunnerError::Backend(e.to_string()))?;
        let Some(cp) = checkpoint else {
            return Ok(None);
        };
        // R2 chokepoint: every replayed write goes through the
        // single `GraphStore::write_slot` path. The R3 idempotent
        // short-circuit absorbs already-current values.
        for (slot, value) in &cp.writes {
            if let Err(e) = self
                .store
                .write_slot(slot, value.clone(), WriteSlotOpts::live())
                .await
            {
                tracing::warn!(
                    run = %run_id,
                    target = ?slot,
                    error = %e,
                    "resume: replay write failed",
                );
            }
        }
        let handle = self
            .launch(
                spec,
                input,
                Some(ResumeContext {
                    run_id,
                    checkpoint: cp,
                }),
            )
            .await;
        Ok(Some(handle))
    }

    /// Shared launch path for [`Self::start`] (fresh `RunId`) and
    /// [`Self::resume`] (loaded `RunId` + initial seq).
    async fn launch(
        &self,
        spec: RunSpec,
        input: SlotMap,
        resume: Option<ResumeContext>,
    ) -> RunHandle {
        // 1. Skill selection — exactly once per outer run (R7).
        //    SPI signature is `select(input, principal)`; the run's
        //    principal comes from RunSpec::with_principal or falls
        //    back to the stage-5 system/Admin default (Phase 4
        //    retires this default once NodeCtx grows a Principal
        //    field).
        let principal_for_selection = spec
            .principal
            .clone()
            .unwrap_or_else(default_system_admin_principal);
        let selection = match self
            .skill_selector
            .select(&input, &principal_for_selection)
            .await
        {
            Ok(sel) => Arc::new(sel),
            Err(e) => {
                tracing::warn!(error = %e, "skill_selector failed; falling back to SkillSelection::None for this run");
                Arc::new(SkillSelection::None)
            }
        };

        // 2. Per-run primitives. `resume` reuses the existing
        //    `RunId`; a fresh start mints a new one.
        let (run, initial_seq, is_resume) = match resume.as_ref() {
            Some(rc) => (rc.run_id, rc.checkpoint.seq, true),
            None => (RunId::new(), 0, false),
        };
        let cancel = RunCancel::new();
        // Stage 6 (D-F3.10): per-run broadcast capacity comes from
        // `RunOpts.event_broadcast_capacity`, not the fixed
        // `FlowRunnerConfig.event_buffer`. Producers never block
        // (`broadcast::Sender::send` evicts the oldest event for the
        // slowest subscriber); the engine spawns its own
        // Lagged-watcher subscriber below to count drops.
        let broadcast_cap = self.run_opts.event_broadcast_capacity.max(1);
        let (events_tx, _) = broadcast::channel::<FlowEvent>(broadcast_cap);

        // Stage 6 (D-F3.10 + D-F3.11): per-run live metrics + the
        // degraded-mode in-memory queue. Both are owned by the
        // propagator's checkpoint hook and observed via
        // `RunHandle::metrics`.
        let metrics = RunMetricsCell::new();
        let degraded_queue =
            DegradedQueue::new(self.run_opts.degraded_queue_capacity, metrics.clone());

        // Stage 6 (D-F3.10): engine-owned `Lagged`-watcher subscriber.
        // Subscribed synchronously *before* spawning so the runner is
        // guaranteed to see every Lagged signal the per-run channel
        // emits; runs as a detached task and exits cleanly when
        // every other sender is dropped.
        drop(spawn_lagged_watcher(&events_tx, metrics.clone()));

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

        // 3b. Phase 3 SPI store: `start` the run on a fresh launch
        //     so subsequent `checkpoint(...)` calls have a parent
        //     row to reference. Resumed runs skip this (the row
        //     already exists from the prior process); a duplicate
        //     `start` would fail the `runs.run_id` PK constraint.
        if let Some(spi) = self.spi_run_store.as_ref() {
            if !is_resume {
                // Stage 8 (D-F3.12): if the caller (typically
                // `FlowAsService`) supplied a `Principal` /
                // `DedupKey` via `RunSpec::with_principal` /
                // `with_dedup_key`, use them; otherwise fall back
                // to the stage-5 `system/Admin` default and a
                // `None` dedup key.
                let principal = spec.principal.clone().unwrap_or(Principal {
                    subject: "system".to_string(),
                    role: starter_spi::auth::Role::Admin,
                    scopes: Vec::new(),
                    tenant_id: None,
                    teams: Vec::new(),
                    extra: serde_json::Value::Null,
                });
                if let Err(e) = spi
                    .start(
                        run,
                        spec.revision,
                        self.run_opts.clone(),
                        principal,
                        spec.dedup_key.clone(),
                    )
                    .await
                {
                    tracing::warn!(
                        run = %run,
                        error = %e,
                        "spi run_store start failed (stage 5: log and continue)",
                    );
                }
            }
        }

        // 4. Subscribe BEFORE spawning so the caller never misses
        //    `RunStarted` (the broadcast channel only delivers to
        //    receivers that exist at send time).
        let initial_rx = events_tx.subscribe();
        let coordinator_rx = events_tx.subscribe();

        let store = self.store.clone();
        let cfg = self.config;
        let spi_for_task = self.spi_run_store.clone();
        let health_for_task = self.health.clone();
        let queue_for_task = degraded_queue.clone();
        let metrics_for_task = metrics.clone();
        let node_state_for_task = self.node_state_store.clone();
        let RunSpec {
            flow,
            revision: _,
            topology,
            seeds,
            terminal_slots,
            principal: _,
            dedup_key: _,
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
                spi_for_task,
                initial_seq,
                health_for_task,
                queue_for_task,
                metrics_for_task,
                node_state_for_task,
            )
            .await
        });

        RunHandle {
            run,
            cancel,
            events_tx,
            initial_rx,
            join,
            metrics,
        }
    }
}

/// Internal state for the resume path threaded into
/// [`FlowRunner::launch`].
struct ResumeContext {
    run_id: RunId,
    checkpoint: RunCheckpoint,
}

/// Errors raised by [`FlowRunner::resume`] (and future per-run
/// engine APIs).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FlowRunnerError {
    /// No Phase 3 SPI [`SpiRunStore`] is attached; resume requires
    /// one. Attach via [`FlowRunner::with_spi_run_store`].
    #[error("no SPI RunStore attached; call FlowRunner::with_spi_run_store first")]
    NoRunStore,
    /// The SPI store call failed. String form of the underlying
    /// [`starter_flow_spi::flow::FlowError`] for portability.
    #[error("spi run_store backend error: {0}")]
    Backend(String),
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
    spi_run_store: Option<Arc<dyn SpiRunStore>>,
    initial_seq: u64,
    health: HealthHandle,
    degraded_queue: Arc<DegradedQueue>,
    metrics: Arc<RunMetricsCell>,
    node_state_store: Arc<dyn starter_flow_spi::state::NodeStateStore>,
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

    // Spawn propagator. When a Phase 3 SPI RunStore is attached,
    // thread a `CheckpointHook` through so per-tick batches land in
    // `run_checkpoints` (D-F3.2). `initial_seq` is the loaded
    // checkpoint's `seq` for a resumed run (0 for a fresh one); the
    // hook adds it to the per-run hop counter so the first
    // post-resume checkpoint carries `seq = initial_seq + 1`,
    // keeping `(run_id, seq)` strictly monotonic across SIGKILL.
    let checkpoint_hook = spi_run_store.as_ref().map(|spi| {
        CheckpointHook::new(
            spi.clone(),
            initial_seq,
            health.clone(),
            degraded_queue.clone(),
            metrics.clone(),
        )
    });
    let skill_arc = {
        let st = state.read().await;
        st.skill_selection
            .clone()
            .unwrap_or_else(|| Arc::new(SkillSelection::None))
    };
    let prop_handle = propagator::spawn_with_checkpoint(
        store.clone(),
        topology,
        cancel.clone(),
        events_tx.clone(),
        run,
        cfg.propagator,
        checkpoint_hook,
        skill_arc,
        // Stage A+B.1: every run threads a `NodeStateStore` into the
        // propagator so node bodies can persist per-instance state
        // through `NodeCtx.state`. Defaults to noop; production
        // surfaces (rubix-agent) pass `Arc<SqliteNodeStateStore>`
        // via `FlowRunner::with_node_state_store`. See
        // `DOCS/flow/scope/node-state.md`.
        node_state_store,
        Some(flow.clone()),
    );

    // Seed writes — these enter through the single chokepoint per R2.
    // Batched so that surface seed adapters that legitimately write
    // multiple input slots of the same node on a single fire (e.g.
    // tool-call nodes with seeded `tool_id` + `input`) wake the node
    // exactly once instead of once per slot. The store's R3
    // idempotent-write short-circuit and per-write `WriteSlotOpts`
    // semantics are preserved per-entry; only the coalesced wake is
    // batch-level. See store impl for the carrier-slot rule.
    if !cancel.is_cancelled() && !seeds.is_empty() {
        let batch: Vec<_> = seeds
            .into_iter()
            .map(|(slot, value)| (slot, value, WriteSlotOpts::live()))
            .collect();
        if let Err(e) = store.write_slot_batch(batch).await {
            tracing::warn!(
                run = %run,
                error = %e,
                "run coordinator seed batch write failed",
            );
        }
    }

    // Completion loop. The coordinator tracks two pieces of state:
    //
    //   * `deadline` — the time-based quiescence window. After every
    //     event we reset it to `now + cfg.quiescence`. When the
    //     deadline elapses *and* no nodes are in flight, the run is
    //     complete.
    //
    //   * `in_flight` — the count of nodes that have emitted
    //     `NodeStarted` but not yet `NodeEmitted` (any number of
    //     emits per started node — propagator emits one per output
    //     slot) or `NodeFailed`. The propagator awaits node bodies
    //     synchronously, so an in-flight node has *no* event traffic
    //     to bump the quiescence deadline while it works. Without
    //     this counter a node body that takes longer than
    //     `cfg.quiescence` (a slow LLM call, a subprocess spawn, a
    //     remote HTTP) races the coordinator into emitting
    //     `RunCompleted` before its output lands in the store, and
    //     surfaces like `FlowAsTool` read the terminal slot too
    //     early. Tracking `NodeStarted - (NodeEmitted-on-this-node ∪
    //     NodeFailed)` removes the race without changing any SPI
    //     surface.
    //
    // `NodeEmitted` carries the node id, so we close the in-flight
    // entry only when the *first* emit for that node arrives — a
    // node with multiple terminal slots still counts as one in-flight
    // unit, matching the propagator's "one invoke per
    // NodeStarted/NodeEmitted+ pair" contract.
    let mut terminal: Option<RunStatus> = None;
    let mut deadline = Instant::now() + cfg.quiescence;
    let mut in_flight: std::collections::HashSet<starter_flow_spi::node::NodeId> =
        std::collections::HashSet::new();

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
                            FlowEvent::NodeStarted { node, .. } => {
                                in_flight.insert(node);
                                deadline = Instant::now() + cfg.quiescence;
                            }
                            FlowEvent::NodeEmitted { node, .. } => {
                                in_flight.remove(&node);
                                deadline = Instant::now() + cfg.quiescence;
                            }
                            FlowEvent::NodeFailed { node, .. } => {
                                in_flight.remove(&node);
                                deadline = Instant::now() + cfg.quiescence;
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
            _ = sleep_until(deadline), if in_flight.is_empty() => {
                // Quiescence with no in-flight nodes: read terminal
                // output and emit RunCompleted.
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

    // Phase 3 SPI: durably record the run outcome (R6 + D-F3.9
    // "final-checkpoint preserved"). Stage 5 logs failures and
    // continues; stage 6 adds retry-with-backoff and the
    // EngineHealth::Degraded transition.
    if let Some(spi) = spi_run_store.as_ref() {
        let outcome = match &final_status {
            RunStatus::Completed => {
                let mut output = SlotMap::new();
                for sr in &terminal_slots {
                    if let Ok(v) = store.read_slot(sr).await {
                        output.insert(format!("{}.{}", sr.node, sr.slot), v);
                    }
                }
                RunOutcome::Completed { output }
            }
            RunStatus::Failed(err) => RunOutcome::Failed { error: err.clone() },
            RunStatus::Cancelled | RunStatus::Pending | RunStatus::Running => RunOutcome::Cancelled,
            // `RunStatus` is `#[non_exhaustive]`; future variants
            // map to `Cancelled` defensively (matches the SqliteRunStore
            // outcome_status() fallback rationale).
            #[allow(unreachable_patterns)]
            _ => RunOutcome::Cancelled,
        };
        if let Err(e) = spi.finish(run, outcome).await {
            tracing::warn!(
                run = %run,
                error = %e,
                "spi run_store finish failed (stage 5: log and continue)",
            );
        }
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
                reads: BTreeMap::new(),
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
            principal: None,
            dedup_key: None,
        };

        let mut handle = runner
            .start(spec, SlotMap::new())
            .await
            .expect("start rejected");
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
            reads: BTreeMap::new(),
            behaviors,
        });

        let spec = RunSpec {
            flow: FlowId::new("flow.test.cancel").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.n", "in"), SlotValue::Int(1))],
            terminal_slots: vec![],
            principal: None,
            dedup_key: None,
        };

        let mut handle = runner
            .start(spec, SlotMap::new())
            .await
            .expect("start rejected");

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
            reads: BTreeMap::new(),
            behaviors,
        });

        let spec = RunSpec {
            flow: FlowId::new("flow.test.cycle").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.n", "in"), SlotValue::Int(1))],
            terminal_slots: vec![],
            principal: None,
            dedup_key: None,
        };

        let mut handle = runner
            .start(spec, SlotMap::new())
            .await
            .expect("start rejected");
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
                _input: &SlotMap,
                _principal: &Principal,
            ) -> Result<SkillSelection, SkillError> {
                self.calls.fetch_add(1, Ordering::SeqCst);
                Ok(SkillSelection::Selected {
                    skill_id: starter_flow_spi::skill::SkillId::new("test.counted").unwrap(),
                    allowed_tools: Vec::new(),
                    resources: Vec::new(),
                    content_hash: "counted".to_string(),
                })
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
            principal: None,
            dedup_key: None,
        };

        let mut handle = runner
            .start(spec, SlotMap::new())
            .await
            .expect("start rejected");
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
        match sel.as_ref() {
            SkillSelection::Selected { content_hash, .. } => {
                assert_eq!(content_hash, "counted");
            }
            other => panic!("expected Selected, got {other:?}"),
        }
    }

    /// Long-running node body whose `invoke` sleeps for `delay`
    /// before returning a single `out` slot. Used to exercise the
    /// in-flight tracker in the run coordinator: the propagator
    /// awaits `invoke` synchronously, so during the sleep there are
    /// no events on the bus to bump the quiescence deadline.
    struct SlowNode {
        kind: KindId,
        delay: Duration,
    }
    impl SlowNode {
        fn new(delay: Duration) -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new("starter.flow.run-test-slow-node").unwrap(),
                delay,
            })
        }
    }
    #[async_trait]
    impl NodeBehavior for SlowNode {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            sleep(self.delay).await;
            let mut out = SlotMap::new();
            out.insert("out".to_owned(), SlotValue::Int(42));
            Ok(out)
        }
    }

    /// A node body that takes longer than `cfg.quiescence` must not
    /// race the run coordinator into emitting `RunCompleted` before
    /// the slot write lands.
    ///
    /// Setup:
    ///   * one node `flow.test.slow` whose `invoke` sleeps 250 ms
    ///   * `cfg.quiescence = 50 ms` (deliberately shorter than the
    ///     sleep so a time-only completion check would fire mid-await)
    ///   * `terminal_slots = [flow.test.slow.out]`
    ///
    /// Expected: status `Completed`, `RunCompleted.output` carries
    /// `flow.test.slow.out -> 42`. Without the in-flight tracker the
    /// coordinator would emit `RunCompleted` with an empty output
    /// map at ~50 ms (well before the 250 ms slot write), and the
    /// terminal-slot read would race the store.
    #[tokio::test]
    async fn slow_node_body_does_not_race_quiescence() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let runner =
            FlowRunner::new(store.clone(), run_store.clone()).with_config(FlowRunnerConfig {
                quiescence: Duration::from_millis(50),
                ..FlowRunnerConfig::default()
            });

        let slow = SlowNode::new(Duration::from_millis(250));
        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(nid("flow.test.slow"), BTreeSet::from(["in".to_owned()]));
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(nid("flow.test.slow"), slow);
        let topology = Arc::new(FlowTopology {
            links: HashMap::new(),
            triggers,
            reads: BTreeMap::new(),
            behaviors,
        });

        let spec = RunSpec {
            flow: FlowId::new("flow.test.slow").unwrap(),
            revision: FlowRevisionId::new(),
            topology,
            seeds: vec![(slot("flow.test.slow", "in"), SlotValue::Int(1))],
            terminal_slots: vec![slot("flow.test.slow", "out")],
            principal: None,
            dedup_key: None,
        };

        let mut handle = runner
            .start(spec, SlotMap::new())
            .await
            .expect("start rejected");

        // Generous outer timeout — should complete in ~300 ms total
        // (250 ms sleep + 50 ms quiescence after the emit).
        let status = timeout(Duration::from_secs(2), &mut handle.join)
            .await
            .expect("coordinator did not exit within 2s")
            .expect("coordinator panicked");
        assert_eq!(status, RunStatus::Completed);

        // The slot store must hold the slow node's output.
        let stored = store
            .read_slot(&slot("flow.test.slow", "out"))
            .await
            .expect("terminal slot was written");
        assert_eq!(stored, SlotValue::Int(42));

        // RunCompleted.output must include the terminal slot — the
        // race we are guarding against is the coordinator reading
        // the store too early and shipping an empty output map.
        let events = drain(&mut handle.initial_rx, Duration::from_millis(100)).await;
        let completed = events
            .iter()
            .find_map(|e| match e {
                FlowEvent::RunCompleted { output, .. } => Some(output),
                _ => None,
            })
            .expect("RunCompleted event was emitted");
        assert_eq!(
            completed.get("flow.test.slow.out"),
            Some(&SlotValue::Int(42)),
            "RunCompleted.output must carry the terminal slot value; got {completed:?}",
        );
    }
}
