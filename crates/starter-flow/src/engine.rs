//! Engine state machine per R12:
//! `Starting → Running → Pausing → Paused → Resuming → Stopping → Stopped`.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / R12. This stage
//! ships the strongly-typed state machine, the writable-output
//! registry the safe-state walk reads from on stop, and the
//! observability hooks (one `tracing::info_span!` per transition, one
//! `tracing::info!` log on entry).
//!
//! **SIGTERM is a bin-level concern.** R12's graceful-shutdown
//! protocol — "(1) stop accepting new triggers, (2) finish in-flight
//! runs with a short timeout, (3) drive writable outputs to safe
//! state, (4) flush the `RunStore` to disk, (5) exit cleanly" — is
//! orchestrated by the host binary. The engine exposes the API the
//! bin will hook ([`Engine::stop`]): the binary listens for SIGTERM
//! (via `tokio::signal::unix`), refuses new triggers at the surface
//! layer (Phase 3 `FlowAsTool` / `FlowAsService`), then calls
//! [`Engine::stop`] which drives `Running → Stopping → Stopped` and
//! walks the writable outputs. No signal handler lives in this crate.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{watch, Mutex, RwLock};

use starter_flow_spi::agent_session::AgentSessionStore as SpiAgentSessionStore;
use starter_flow_spi::flow::{EngineHealth, RunStore as SpiRunStore};
use starter_flow_spi::graph::{GraphStore, WriteSlotOpts};
use starter_flow_spi::node::{SlotRef, SlotValue};

use crate::health::HealthHandle;
use crate::registry::{FlowRegistry, NodeKindRegistry};

/// Engine state — lifted verbatim from rubix RUNTIME per R12.
///
/// Encoded as a plain `Copy` enum rather than a Rust type-state on
/// `Engine` itself: the engine is observed concurrently from many
/// places (the surface adapters, the propagator, the bin's signal
/// handler, traces), so we want one runtime value behind a
/// [`watch::Sender`] rather than a compile-time phantom that erases
/// at the API boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EngineState {
    /// Engine constructed but not yet up. Initial state.
    Starting,
    /// Engine up — accepting triggers, propagator running.
    Running,
    /// Engine asked to pause; finishing in-flight work.
    Pausing,
    /// Engine paused — no new triggers accepted, propagator idle.
    Paused,
    /// Engine asked to resume from a paused state.
    Resuming,
    /// Engine asked to stop; finishing in-flight work and driving
    /// writable outputs to safe state.
    Stopping,
    /// Engine fully stopped. Terminal — re-use requires a new
    /// [`Engine`] value.
    Stopped,
}

impl EngineState {
    /// Whether `next` is reachable from `self` per the SCOPE R12
    /// transition matrix.
    ///
    /// Legal transitions (every other pair is rejected):
    ///
    /// - `Starting → Running` (normal startup completion)
    /// - `Starting → Stopping` (abort startup; the bin tears the
    ///   engine down before it ever served traffic)
    /// - `Running → Pausing` (operator-driven pause)
    /// - `Pausing → Paused` (pause complete)
    /// - `Paused → Resuming` (operator-driven resume)
    /// - `Resuming → Running` (resume complete)
    /// - `Running → Stopping`, `Pausing → Stopping`,
    ///   `Paused → Stopping`, `Resuming → Stopping` (operator-driven
    ///   stop reaches the graceful-shutdown machinery from any
    ///   non-terminal state)
    /// - `Stopping → Stopped` (stop complete)
    pub fn can_transition_to(self, next: EngineState) -> bool {
        use EngineState::*;
        matches!(
            (self, next),
            (Starting, Running)
                | (Starting, Stopping)
                | (Running, Pausing)
                | (Pausing, Paused)
                | (Paused, Resuming)
                | (Resuming, Running)
                | (Running, Stopping)
                | (Pausing, Stopping)
                | (Paused, Stopping)
                | (Resuming, Stopping)
                | (Stopping, Stopped)
        )
    }
}

impl std::fmt::Display for EngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EngineState::Starting => "starting",
            EngineState::Running => "running",
            EngineState::Pausing => "pausing",
            EngineState::Paused => "paused",
            EngineState::Resuming => "resuming",
            EngineState::Stopping => "stopping",
            EngineState::Stopped => "stopped",
        };
        f.write_str(s)
    }
}

/// Errors raised by the engine state machine.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineError {
    /// An attempted state transition is not in the R12 transition
    /// matrix.
    #[error("illegal engine state transition: {from} → {to}")]
    IllegalTransition {
        /// The state the engine was in.
        from: EngineState,
        /// The state the caller asked to move to.
        to: EngineState,
    },
}

/// A writable output that the engine drives to its declared safe
/// state on stop (R12).
///
/// SCOPE R3 + R12: the `safe_state` policy is declared on a node's
/// config slot — `hold-last`, `fail-safe(value)`, or `release`. Phase
/// 2 ships the `fail-safe(value)` flavour through this trait (a fixed
/// [`SlotValue`] driven into a [`SlotRef`]); `hold-last` is implicit
/// (don't register anything — the slot keeps its last value) and
/// `release` is kind-validated and so lands with the kinds that
/// support it (Phase 5+).
///
/// The default [`Self::write_safe_state`] implementation writes
/// `safe_state()` to `slot()` via the single [`GraphStore::write_slot`]
/// chokepoint. The safe-state drive **does** publish a `SlotChanged`
/// event — operators need safe-state in audit (SCOPE R2 explicit
/// callout: "the engine's safe-state drive (R12) *does* publish
/// `SlotChanged` … both flags are visible in the per-write tracing
/// span"). Override the method only if a kind needs a protocol-level
/// handshake on top of the slot write.
#[async_trait::async_trait]
pub trait WritableOutput: Send + Sync + 'static {
    /// The slot this output writes to.
    fn slot(&self) -> SlotRef;

    /// The value to drive on engine / flow stop.
    fn safe_state(&self) -> SlotValue;

    /// Drive the safe-state value through the single
    /// [`GraphStore::write_slot`] chokepoint.
    ///
    /// Default impl: `store.write_slot(self.slot(), self.safe_state(),
    /// WriteSlotOpts::live())`. Kinds that need to do additional
    /// protocol-level work (BACnet priority release, etc.) override
    /// this method but **must** still go through `write_slot` for the
    /// slot side-effect.
    async fn write_safe_state(
        &self,
        store: &dyn GraphStore,
    ) -> Result<(), starter_flow_spi::graph::GraphError> {
        let slot = self.slot();
        let value = self.safe_state();
        store.write_slot(&slot, value, WriteSlotOpts::live()).await
    }
}

/// THE engine.
///
/// Owns the [`GraphStore`], the optional propagator handle (Phase 2
/// stage 4 spawned one of these per run; later stages will move the
/// handle into the per-run record), the two registries from stage 5,
/// the set of [`WritableOutput`]s the safe-state walk visits, and the
/// `watch::Sender<EngineState>` that callers subscribe to for state
/// changes.
///
/// `start` / `stop` / `pause` / `resume` are the only public ways to
/// move the state machine. Every transition is logged at `info!` and
/// emits a `tracing::info_span!` named `engine.transition`.
pub struct Engine {
    /// The single in-engine [`GraphStore`] all writes funnel through.
    pub store: Arc<dyn GraphStore>,
    /// Registry of node-kind behaviours, populated at engine boot.
    pub node_kinds: Arc<NodeKindRegistry>,
    /// Registry of flow definitions and revisions.
    pub flows: Arc<FlowRegistry>,
    /// State broadcast — every subscriber sees every transition.
    state_tx: watch::Sender<EngineState>,
    /// Optional propagator task handle. Phase 2 stage 4 spawned one
    /// of these per run; this stage tracks the most recent so
    /// [`Self::stop`] can join on it during the graceful walk. A
    /// `Mutex<Option<...>>` so `stop` can `take()` and `await` the
    /// handle without holding the state lock.
    propagator: Mutex<Option<tokio::task::JoinHandle<()>>>,
    /// Writable outputs the safe-state walk drives on stop (R12).
    writables: RwLock<Vec<Arc<dyn WritableOutput>>>,
    /// Phase 3 SPI [`SpiRunStore`] for run lifecycle + per-tick
    /// checkpoint persistence (R6, D-F3.2). The engine itself does
    /// not call this; it is held here so the per-run constructor
    /// (`FlowRunner` in [`crate::run`]) can pull it via
    /// [`Self::run_store`] and thread it into the run. `None` means
    /// the engine runs Phase-2-style in-memory only.
    run_store: Option<Arc<dyn SpiRunStore>>,
    /// MEMORY.md M1 [`SpiAgentSessionStore`] for agent turn /
    /// artifact persistence. Separate from
    /// [`Self::run_store`] (flow runs) and from the legacy
    /// `SessionStore` key/value seam; this one backs the
    /// `ai-agent` loop and surface artifacts (page-builder trees,
    /// chat summaries, ...). `None` means agent surfaces fall
    /// back to today's ephemeral / stateless behaviour (M13).
    agent_session_store: Option<Arc<dyn SpiAgentSessionStore>>,
    /// MEMORY.md §M9 / Phase M-E retention policies, keyed by
    /// session `kind` (e.g. `"page-builder"`, `"chat"`). Empty
    /// (default) means
    /// [`starter_flow_spi::agent_session::RetentionPolicy::KeepForever`]
    /// for every kind. Hosts run the sweep via
    /// [`Self::sweep_agent_session_retention`] from their own
    /// scheduled task; the engine does not own the clock.
    agent_session_retention:
        std::collections::HashMap<String, starter_flow_spi::agent_session::RetentionPolicy>,
    /// Engine-level health flag (D-F3.11). Backed by an `AtomicU8`
    /// so [`Self::health`] is lock-free. Shared with every
    /// [`crate::run::FlowRunner`] launched off this engine via
    /// [`Self::health_handle`]; the propagator's per-tick
    /// retry-with-backoff loop flips this on degrade / recovery.
    health: HealthHandle,
    /// Phase 4 D-F4.3: optional `AiRunnerRegistry` the `ai-agent`
    /// node body resolves its `provider_id` config slot against.
    /// `None` means no providers registered — the body errors with
    /// `NodeError::Domain { code: "provider_not_registered" }` on
    /// invoke. Engine consumers attach via
    /// [`Self::with_ai_runner_registry`].
    ai_runners: Option<Arc<dyn starter_flow_spi::ai_runner::AiRunnerRegistry>>,
    /// Phase 4 D-F4.4: engine-level `SkillSelector` invoked once
    /// per [`crate::run::FlowRunner::start`]. Defaults to
    /// [`starter_flow_spi::skill::NullSkillSelector`] (returns
    /// [`starter_flow_spi::skill::SkillSelection::None`]). Hosts
    /// attach a real selector via [`Self::with_skill_selector`].
    skill_selector: Arc<dyn starter_flow_spi::skill::SkillSelector>,
    /// HR-3 (`DOCS/flow/scope/hot-reload.md`): optional
    /// [`crate::definition::DefinitionManager`] — the HR-1
    /// publish chokepoint, HR-2 active-topology registry, HR-3
    /// observability surface and definition bus. `None` means
    /// the engine runs without hot-reload: hand-built
    /// [`crate::propagator::FlowTopology`]s come in through
    /// [`crate::run::FlowRunner`] directly, the way Phase 2
    /// demos already do. Hosts wanting hot-reload attach a
    /// manager via [`Self::with_definition_manager`].
    definitions: Option<Arc<crate::definition::DefinitionManager>>,
}

impl Engine {
    /// Construct a new [`Engine`] in the [`EngineState::Starting`]
    /// state. Call [`Self::start`] to transition to `Running`.
    pub fn new(store: Arc<dyn GraphStore>) -> Self {
        let (state_tx, _state_rx) = watch::channel(EngineState::Starting);
        Self {
            store,
            node_kinds: Arc::new(NodeKindRegistry::new()),
            flows: Arc::new(FlowRegistry::new()),
            state_tx,
            propagator: Mutex::new(None),
            writables: RwLock::new(Vec::new()),
            run_store: None,
            agent_session_store: None,
            agent_session_retention: std::collections::HashMap::new(),
            health: HealthHandle::new(),
            ai_runners: None,
            skill_selector: Arc::new(starter_flow_spi::skill::NullSkillSelector),
            definitions: None,
        }
    }

    /// Attach a Phase 4 [`AiRunnerRegistry`](starter_flow_spi::ai_runner::AiRunnerRegistry)
    /// — the `ai-agent` node body resolves its mandatory
    /// `provider_id` config slot against this. D-F4.3. Self-by-value
    /// builder mirrors [`Self::with_run_store`].
    pub fn with_ai_runner_registry(
        mut self,
        registry: Arc<dyn starter_flow_spi::ai_runner::AiRunnerRegistry>,
    ) -> Self {
        self.ai_runners = Some(registry);
        self
    }

    /// Borrow the attached [`AiRunnerRegistry`] if any.
    pub fn ai_runners(&self) -> Option<&Arc<dyn starter_flow_spi::ai_runner::AiRunnerRegistry>> {
        self.ai_runners.as_ref()
    }

    /// Attach a Phase 4 [`SkillSelector`](starter_flow_spi::skill::SkillSelector)
    /// — invoked exactly once per outer flow run by
    /// [`crate::run::FlowRunner::start`], with the result threaded
    /// through every `NodeCtx` as
    /// [`starter_flow_spi::skill::SkillSelection`]. D-F4.4.
    /// Self-by-value builder mirrors [`Self::with_run_store`].
    pub fn with_skill_selector(
        mut self,
        selector: Arc<dyn starter_flow_spi::skill::SkillSelector>,
    ) -> Self {
        self.skill_selector = selector;
        self
    }

    /// Clone the engine's [`SkillSelector`]. Callers wiring per-run
    /// constructors (e.g. [`crate::run::FlowRunner`]) clone this
    /// `Arc` into the run.
    pub fn skill_selector(&self) -> Arc<dyn starter_flow_spi::skill::SkillSelector> {
        self.skill_selector.clone()
    }

    /// Attach a Phase 3 SPI [`SpiRunStore`] for run lifecycle +
    /// per-tick checkpoint persistence (R6, D-F3.2). Builder hook
    /// per the Phase 3 SCOPE: "the engine's per-run propagator
    /// gains an `Option<Arc<dyn RunStore>>` slot threaded through
    /// `Engine::with_run_store(…)`". When no store is attached
    /// the engine behaves exactly as it does today (in-memory
    /// Phase-2 substrate).
    pub fn with_run_store(mut self, store: Arc<dyn SpiRunStore>) -> Self {
        self.run_store = Some(store);
        self
    }

    /// Replace the engine's [`NodeKindRegistry`] with a shared
    /// `Arc` — useful when a [`crate::definition::DefinitionManager`]
    /// already holds the registry that hosts want to wire through
    /// [`Self::register_kind`] / [`Self::deregister_kind`]. The
    /// engine creates a fresh empty registry in [`Self::new`]; this
    /// builder lets callers replace it with the same `Arc` they
    /// pass to [`crate::definition::DefinitionManager::new`] so the
    /// HR-6 walks resolve against the registry they just mutated.
    pub fn with_node_kinds(mut self, kinds: Arc<NodeKindRegistry>) -> Self {
        self.node_kinds = kinds;
        self
    }

    /// Borrow the attached [`SpiRunStore`] if any. Callers wiring
    /// per-run constructors (e.g. [`crate::run::FlowRunner`]) clone
    /// this `Arc` into the run.
    pub fn run_store(&self) -> Option<&Arc<dyn SpiRunStore>> {
        self.run_store.as_ref()
    }

    /// Attach an [`SpiAgentSessionStore`] (DOCS/agent/MEMORY.md
    /// M1) for agent conversation + artifact persistence. The
    /// engine itself does not call this; surfaces (the page
    /// builder, chat routes, etc.) pull it via
    /// [`Self::agent_session_store`] to persist turns and fetch
    /// the latest artifact on page reload. `None` (default)
    /// preserves today's stateless `/api/builder/stream`
    /// behaviour per MEMORY.md M13.
    pub fn with_agent_session_store(mut self, store: Arc<dyn SpiAgentSessionStore>) -> Self {
        self.agent_session_store = Some(store);
        self
    }

    /// Borrow the attached [`SpiAgentSessionStore`] if any.
    pub fn agent_session_store(&self) -> Option<&Arc<dyn SpiAgentSessionStore>> {
        self.agent_session_store.as_ref()
    }

    /// Register a [`RetentionPolicy`] for one session `kind`
    /// (DOCS/agent/MEMORY.md §M9 / Phase M-E). Re-attaching the
    /// same `kind` replaces the prior policy. Self-by-value
    /// mirrors the other `with_*` hooks.
    ///
    /// The engine does **not** spawn a sweep task on its own —
    /// hosts call [`Self::sweep_agent_session_retention`] from
    /// whichever scheduler they already run (a `tokio::interval`
    /// in `main.rs`, a cron, a maintenance command). This keeps
    /// the engine free of an opinionated runtime dependency and
    /// makes the sweep trivial to drive in tests.
    ///
    /// [`RetentionPolicy`]: starter_flow_spi::agent_session::RetentionPolicy
    pub fn with_agent_session_retention(
        mut self,
        kind: impl Into<String>,
        policy: starter_flow_spi::agent_session::RetentionPolicy,
    ) -> Self {
        self.agent_session_retention.insert(kind.into(), policy);
        self
    }

    /// Borrow the configured retention policies, keyed by
    /// session `kind`.
    pub fn agent_session_retention(
        &self,
    ) -> &std::collections::HashMap<String, starter_flow_spi::agent_session::RetentionPolicy> {
        &self.agent_session_retention
    }

    /// Run one retention sweep across every configured `(kind,
    /// policy)` pair against the attached agent-session store.
    /// Returns the merged [`RetentionSweepReport`].
    ///
    /// `now` is supplied so deterministic tests pin a cutoff;
    /// production callers pass [`chrono::Utc::now`]. Returns
    /// [`Ok(None)`] when no agent-session store is attached or
    /// no policies are registered (callers can no-op without
    /// branching on configuration).
    ///
    /// [`RetentionSweepReport`]: starter_flow_spi::agent_session::RetentionSweepReport
    pub async fn sweep_agent_session_retention(
        &self,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<
        Option<starter_flow_spi::agent_session::RetentionSweepReport>,
        starter_flow_spi::agent_session::AgentSessionError,
    > {
        let Some(store) = self.agent_session_store.as_ref() else {
            return Ok(None);
        };
        if self.agent_session_retention.is_empty() {
            return Ok(None);
        }
        let mut total = starter_flow_spi::agent_session::RetentionSweepReport::default();
        for (kind, policy) in &self.agent_session_retention {
            let report = store.sweep_retention(kind, policy, now).await?;
            total = total.merge(report);
        }
        Ok(Some(total))
    }

    /// Attach the hot-reload [`crate::definition::DefinitionManager`]
    /// per `DOCS/flow/scope/hot-reload.md` HR-3. The manager owns
    /// the publish chokepoint, the active-topology registry, the
    /// definition bus, and the definition-layer metrics cell.
    /// Hosts that don't want hot-reload simply don't attach one.
    ///
    /// Self-by-value builder mirrors the other `with_*` hooks.
    pub fn with_definition_manager(
        mut self,
        manager: Arc<crate::definition::DefinitionManager>,
    ) -> Self {
        self.definitions = Some(manager);
        self
    }

    /// Borrow the attached
    /// [`crate::definition::DefinitionManager`] if any.
    pub fn definitions(&self) -> Option<&Arc<crate::definition::DefinitionManager>> {
        self.definitions.as_ref()
    }

    /// Subscribe to the definition bus, if a
    /// [`crate::definition::DefinitionManager`] is attached.
    /// Returns `None` when hot-reload isn't wired \u2014 callers must
    /// degrade gracefully (no events to surface).
    pub fn definition_events(
        &self,
    ) -> Option<tokio::sync::broadcast::Receiver<starter_flow_spi::definition::FlowDefinitionEvent>>
    {
        self.definitions.as_ref().map(|m| m.subscribe())
    }

    /// Register a node kind through the engine's chokepoint
    /// (HR-6 / HR8 first paragraph).
    ///
    /// Forwards to [`NodeKindRegistry::register`]. If a
    /// [`crate::definition::DefinitionManager`] is attached, the
    /// engine subsequently invokes
    /// [`crate::definition::DefinitionManager::on_kind_registered`]
    /// so any flow currently in `ResolveFailed` for this kind can
    /// remount. Callers who hold the registry directly are still
    /// supported, but they MUST drive the remount walk themselves
    /// — preferring this method keeps the host honest.
    pub async fn register_kind(
        &self,
        behavior: Arc<dyn starter_flow_spi::node::NodeBehavior>,
    ) -> Result<(), crate::registry::RegistryError> {
        let kind = behavior.kind_id().clone();
        self.node_kinds.register(behavior).await?;
        if let Some(defs) = self.definitions.as_ref() {
            let remounted = defs.on_kind_registered(&kind).await;
            tracing::debug!(
                target: "starter_flow::engine",
                kind = %kind,
                remounted,
                "register_kind: remount walk complete"
            );
        }
        Ok(())
    }

    /// Host-only variant: register a kind under the reserved
    /// `starter.flow.*` prefix. Forwards to
    /// [`NodeKindRegistry::register_builtin`] then fires the same
    /// HR-6 remount walk as [`Self::register_kind`].
    pub async fn register_builtin_kind(
        &self,
        behavior: Arc<dyn starter_flow_spi::node::NodeBehavior>,
    ) -> Result<(), crate::registry::RegistryError> {
        let kind = behavior.kind_id().clone();
        self.node_kinds.register_builtin(behavior).await?;
        if let Some(defs) = self.definitions.as_ref() {
            let _ = defs.on_kind_registered(&kind).await;
        }
        Ok(())
    }

    /// Deregister a node kind through the engine's chokepoint
    /// (HR-6 / HR8 second paragraph).
    ///
    /// Forwards to [`NodeKindRegistry::deregister`]. If a
    /// [`crate::definition::DefinitionManager`] is attached, the
    /// engine subsequently invokes
    /// [`crate::definition::DefinitionManager::on_kind_deregistered`]
    /// to revoke every active topology that references the kind,
    /// cancel in-flight runs per each flow's `apply_policy`, and
    /// transition affected flows to `ResolveFailed`.
    pub async fn deregister_kind(
        &self,
        kind: &starter_flow_spi::node::KindId,
    ) -> Result<(), crate::registry::RegistryError> {
        // Drive the manager walk *first* so in-flight runs get
        // their cancel signal while behaviors are still reachable
        // through both the registry and the active-topology
        // snapshot. The registry drop then races only with the
        // snapshot Arcs held by drain-policy runs, which is the
        // HR8 memory-safety contract.
        if let Some(defs) = self.definitions.as_ref() {
            let revoked = defs.on_kind_deregistered(kind).await;
            tracing::debug!(
                target: "starter_flow::engine",
                kind = %kind,
                revoked,
                "deregister_kind: revoke walk complete"
            );
        }
        self.node_kinds.deregister(kind).await
    }

    /// Read the engine's current health (D-F3.11). Lock-free; backed
    /// by an `AtomicU8`. Returns
    /// [`EngineHealth::Healthy`] under normal operation and
    /// [`EngineHealth::Degraded`] after the per-run propagator has
    /// observed five consecutive `RunStore::checkpoint` failures
    /// (the engine returns to `Healthy` once the next checkpoint
    /// succeeds and the per-run in-memory queue is drained).
    pub fn health(&self) -> EngineHealth {
        self.health.get()
    }

    /// Clone the engine's shared health handle. Hand this to a
    /// [`crate::run::FlowRunner`] via
    /// [`crate::run::FlowRunner::with_health_handle`] so the
    /// runner's `start(...)` rejection check and the propagator's
    /// degrade/recover transitions see the same flag.
    pub fn health_handle(&self) -> HealthHandle {
        self.health.clone()
    }

    /// Borrow the current state.
    pub fn state(&self) -> EngineState {
        *self.state_tx.borrow()
    }

    /// Subscribe to state transitions. The returned receiver yields
    /// every state the engine moves through; subscribers can `select!`
    /// against it for liveness, readiness, and graceful-shutdown
    /// hooks.
    pub fn subscribe_state(&self) -> watch::Receiver<EngineState> {
        self.state_tx.subscribe()
    }

    /// Register a [`WritableOutput`] whose `safe_state` is driven on
    /// stop (R12). Order of registration is the order the safe-state
    /// walk visits them.
    pub async fn register_writable(&self, output: Arc<dyn WritableOutput>) {
        self.writables.write().await.push(output);
    }

    /// Replace the tracked propagator task handle.
    ///
    /// The propagator spawned by [`crate::propagator::spawn`] returns
    /// a [`tokio::task::JoinHandle<()>`]; the engine tracks the most
    /// recent one here so [`Self::stop`] can join on it during the
    /// graceful-shutdown walk. Returns the previously-tracked handle
    /// if any.
    pub async fn set_propagator(
        &self,
        handle: tokio::task::JoinHandle<()>,
    ) -> Option<tokio::task::JoinHandle<()>> {
        self.propagator.lock().await.replace(handle)
    }

    /// `Starting → Running`. Idempotent on `Running` is **not**
    /// supported — a second call returns [`EngineError::IllegalTransition`].
    ///
    /// If a [`crate::definition::DefinitionManager`] is attached
    /// (via [`Self::with_definition_manager`]), the start walk
    /// kicks off the HR-5 boot-resume per
    /// `DOCS/flow/scope/hot-reload.md`: every flow known to the
    /// `FlowStore` is loaded, resolved, and either installed into
    /// [`crate::definition::ActiveTopologies`] or surfaced as a
    /// [`starter_flow_spi::definition::FlowDefinitionEvent::ResolveFailed`].
    /// A boot whose `FlowStore::list` errors logs the failure
    /// and continues into `Running` — the engine boots degraded
    /// rather than refusing to start, matching the
    /// `EngineHealth::Degraded` posture from D-F3.11.
    pub async fn start(&self) -> Result<(), EngineError> {
        self.transition(EngineState::Running)?;
        if let Some(manager) = self.definitions.as_ref() {
            match manager.boot_resume().await {
                Ok(report) => {
                    tracing::info!(
                        mounted = report.mounted,
                        failed = report.failed,
                        skipped = report.skipped,
                        "engine.start: boot_resume complete",
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        error = %err,
                        "engine.start: boot_resume failed; continuing in degraded mode",
                    );
                }
            }
        }
        Ok(())
    }

    /// `Running → Pausing → Paused`. Each leg is a separate transition
    /// recorded in the state stream and the tracing log.
    pub async fn pause(&self) -> Result<(), EngineError> {
        self.transition(EngineState::Pausing)?;
        self.transition(EngineState::Paused)
    }

    /// `Paused → Resuming → Running`. Each leg is recorded separately.
    pub async fn resume(&self) -> Result<(), EngineError> {
        self.transition(EngineState::Resuming)?;
        self.transition(EngineState::Running)
    }

    /// Graceful stop per R12: move to `Stopping`, walk every
    /// registered [`WritableOutput`] and drive its `safe_state`
    /// through the single [`GraphStore::write_slot`] chokepoint, join
    /// the tracked propagator task if any, then move to `Stopped`.
    ///
    /// R12 step (1) "stop accepting new triggers" is enforced by the
    /// surface layer once it observes `state() != Running` — the
    /// engine is the source of truth for the flag but does not own
    /// the trigger endpoints. R12 step (4) "flush `RunStore` to disk"
    /// lands in Phase 3 when `RunStore` ships; this stage stubs the
    /// hook by ordering the transitions correctly.
    pub async fn stop(&self) -> Result<(), EngineError> {
        // Step (1): move into `Stopping`. The transition matrix lets
        // us in from `Starting`, `Running`, `Pausing`, `Paused`, and
        // `Resuming`.
        self.transition(EngineState::Stopping)?;

        // Step (3): drive writable outputs to safe state. Snapshot
        // the list under the read lock so a `WritableOutput::write_safe_state`
        // impl that re-enters the engine (in tests or future ext-flow
        // wiring) doesn't deadlock.
        let writables: Vec<Arc<dyn WritableOutput>> =
            self.writables.read().await.iter().cloned().collect();
        for w in &writables {
            if let Err(err) = w.write_safe_state(self.store.as_ref()).await {
                tracing::warn!(
                    slot = ?w.slot(),
                    error = %err,
                    "engine.stop: safe-state write failed",
                );
            }
        }

        // Step (2): finish in-flight work. The propagator task exits
        // on its own once its subscription stream is dropped or its
        // Cancel fires; here we join on the handle if one is tracked.
        // Per-run cancellation is the propagator's responsibility
        // (R13) — the engine does not flip individual run Cancels
        // here because per-run handles aren't tracked yet (Phase 2
        // stage 7 lands `FlowRunner`).
        if let Some(handle) = self.propagator.lock().await.take() {
            // Best-effort: abort cleans up the task without hanging
            // `stop` if a misbehaved propagator never exits on its
            // own.
            handle.abort();
            let _ = handle.await;
        }

        // Step (5): land in `Stopped`.
        self.transition(EngineState::Stopped)
    }

    /// Move into `next`, returning [`EngineError::IllegalTransition`]
    /// if the move is not in the R12 transition matrix. Logs the
    /// transition at `info!` and opens an `engine.transition` span.
    fn transition(&self, next: EngineState) -> Result<(), EngineError> {
        let current = *self.state_tx.borrow();
        if !current.can_transition_to(next) {
            tracing::warn!(
                from = %current,
                to = %next,
                "engine.transition refused: not in R12 matrix",
            );
            return Err(EngineError::IllegalTransition {
                from: current,
                to: next,
            });
        }
        let span = tracing::info_span!("engine.transition", from = %current, to = %next);
        let _enter = span.enter();
        tracing::info!(from = %current, to = %next, "engine state transition");
        // `send_replace` ignores receiver count and always swaps the
        // value, which is what we want — there is exactly one
        // authoritative state per engine.
        let _ = self.state_tx.send_replace(next);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::graph::InMemoryGraphStore;

    /// Full enumeration of every state pair: legal pairs must be
    /// allowed by [`EngineState::can_transition_to`]; every other pair
    /// must be refused. Mirrors the R12 transition matrix in the
    /// SCOPE document.
    #[test]
    fn transition_matrix_legal_and_illegal() {
        use EngineState::*;
        let legal: &[(EngineState, EngineState)] = &[
            (Starting, Running),
            (Starting, Stopping),
            (Running, Pausing),
            (Pausing, Paused),
            (Paused, Resuming),
            (Resuming, Running),
            (Running, Stopping),
            (Pausing, Stopping),
            (Paused, Stopping),
            (Resuming, Stopping),
            (Stopping, Stopped),
        ];
        let all = [
            Starting, Running, Pausing, Paused, Resuming, Stopping, Stopped,
        ];

        for from in all {
            for to in all {
                let want_legal = legal.contains(&(from, to));
                assert_eq!(
                    from.can_transition_to(to),
                    want_legal,
                    "transition {from} → {to}: expected legal={want_legal}",
                );
            }
        }
    }

    /// Drive the happy-path lifecycle through every legal API call
    /// and assert each leg succeeds. Also asserts that calling each
    /// API in the wrong state returns [`EngineError::IllegalTransition`].
    #[tokio::test]
    async fn engine_lifecycle_happy_path_and_illegal_calls_return_typed_error() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store);

        assert_eq!(engine.state(), EngineState::Starting);

        // pause / resume are illegal from Starting. (stop IS legal
        // from Starting; tested separately below.)
        let err = engine.pause().await.unwrap_err();
        assert!(matches!(
            err,
            EngineError::IllegalTransition {
                from: EngineState::Starting,
                to: EngineState::Pausing,
            }
        ));
        let err = engine.resume().await.unwrap_err();
        assert!(matches!(
            err,
            EngineError::IllegalTransition {
                from: EngineState::Starting,
                to: EngineState::Resuming,
            }
        ));

        engine.start().await.unwrap();
        assert_eq!(engine.state(), EngineState::Running);

        // Double-start is illegal.
        let err = engine.start().await.unwrap_err();
        assert!(matches!(
            err,
            EngineError::IllegalTransition {
                from: EngineState::Running,
                to: EngineState::Running,
            }
        ));

        // Resume-from-Running is illegal.
        let err = engine.resume().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));

        engine.pause().await.unwrap();
        assert_eq!(engine.state(), EngineState::Paused);

        // Pause-from-Paused is illegal.
        let err = engine.pause().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));

        engine.resume().await.unwrap();
        assert_eq!(engine.state(), EngineState::Running);

        engine.stop().await.unwrap();
        assert_eq!(engine.state(), EngineState::Stopped);

        // Everything is illegal from Stopped.
        let err = engine.start().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));
        let err = engine.pause().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));
        let err = engine.resume().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));
        let err = engine.stop().await.unwrap_err();
        assert!(matches!(err, EngineError::IllegalTransition { .. }));
    }

    /// `Starting → Stopping → Stopped` is legal: the bin may tear the
    /// engine down before it ever served traffic.
    #[tokio::test]
    async fn stop_directly_from_starting() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store);
        engine.stop().await.unwrap();
        assert_eq!(engine.state(), EngineState::Stopped);
    }

    /// Fake [`WritableOutput`] that records every safe-state write
    /// for the R12 walk test. Wraps the default impl so the slot
    /// write still hits the [`GraphStore`] chokepoint — the recording
    /// is purely observational.
    struct FakeWritable {
        slot: SlotRef,
        safe_state: SlotValue,
        writes: Arc<AtomicUsize>,
    }

    #[async_trait::async_trait]
    impl WritableOutput for FakeWritable {
        fn slot(&self) -> SlotRef {
            self.slot.clone()
        }
        fn safe_state(&self) -> SlotValue {
            self.safe_state.clone()
        }
        async fn write_safe_state(
            &self,
            store: &dyn GraphStore,
        ) -> Result<(), starter_flow_spi::graph::GraphError> {
            self.writes.fetch_add(1, Ordering::SeqCst);
            store
                .write_slot(&self.slot(), self.safe_state(), WriteSlotOpts::live())
                .await
        }
    }

    /// `engine.stop` walks every registered [`WritableOutput`] and
    /// writes its safe state through the [`GraphStore`] chokepoint per
    /// R12. Two outputs are registered; both must be written exactly
    /// once, and both target slots must reflect the safe value in the
    /// store after `stop` returns.
    #[tokio::test]
    async fn stop_walks_writable_nodes_and_writes_safe_state() {
        use starter_flow_spi::node::NodeId;

        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store.clone());

        let slot_a = SlotRef::new(NodeId::new("flow.test.out_a").unwrap(), "value");
        let slot_b = SlotRef::new(NodeId::new("flow.test.out_b").unwrap(), "value");

        // Seed the store with the "live" values so the safe-state
        // writes are observable as overwrites (not first writes).
        store
            .write_slot(&slot_a, SlotValue::Int(42), WriteSlotOpts::live())
            .await
            .unwrap();
        store
            .write_slot(
                &slot_b,
                SlotValue::String("hot".into()),
                WriteSlotOpts::live(),
            )
            .await
            .unwrap();

        let writes_a = Arc::new(AtomicUsize::new(0));
        let writes_b = Arc::new(AtomicUsize::new(0));
        engine
            .register_writable(Arc::new(FakeWritable {
                slot: slot_a.clone(),
                safe_state: SlotValue::Int(0),
                writes: writes_a.clone(),
            }))
            .await;
        engine
            .register_writable(Arc::new(FakeWritable {
                slot: slot_b.clone(),
                safe_state: SlotValue::String("safe".into()),
                writes: writes_b.clone(),
            }))
            .await;

        engine.start().await.unwrap();
        engine.stop().await.unwrap();
        assert_eq!(engine.state(), EngineState::Stopped);

        assert_eq!(writes_a.load(Ordering::SeqCst), 1);
        assert_eq!(writes_b.load(Ordering::SeqCst), 1);

        assert_eq!(store.read_slot(&slot_a).await.unwrap(), SlotValue::Int(0));
        assert_eq!(
            store.read_slot(&slot_b).await.unwrap(),
            SlotValue::String("safe".into()),
        );
    }

    /// The state watch reflects the terminal state after a full
    /// lifecycle. (`watch` is value-latest, not edge-history, so we
    /// assert the most-recent observation rather than a transcript.)
    #[tokio::test]
    async fn state_watch_reflects_terminal_state() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store);
        let rx = engine.subscribe_state();
        assert_eq!(*rx.borrow(), EngineState::Starting);

        engine.start().await.unwrap();
        engine.pause().await.unwrap();
        engine.resume().await.unwrap();
        engine.stop().await.unwrap();

        assert_eq!(*rx.borrow(), EngineState::Stopped);
    }

    // ---------------------------------------------------------
    // MEMORY.md §M9 / Phase M-E retention wiring
    // ---------------------------------------------------------

    /// `sweep_agent_session_retention` is a no-op when nothing is
    /// configured — both the store-missing and the policies-empty
    /// branches return `Ok(None)` without errors. This keeps host
    /// schedulers branch-free.
    #[tokio::test]
    async fn sweep_retention_returns_none_without_store_or_policy() {
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store);
        let report = engine
            .sweep_agent_session_retention(chrono::Utc::now())
            .await
            .unwrap();
        assert!(report.is_none(), "no store + no policy ⇒ no sweep");
    }

    /// Calling the sweeper through the engine dispatches to the
    /// store once per configured `(kind, policy)` pair and merges
    /// the per-kind reports. A tiny in-test fake counts invocations
    /// — exhaustive store-level behaviour is tested in
    /// `crates/starter-store-sqlite/tests/agent_sessions.rs`.
    #[tokio::test]
    async fn sweep_retention_dispatches_per_kind() {
        use starter_flow_spi::agent_session::{
            AgentSession, AgentSessionId, AgentSessionResult, AgentSessionStore, Artifact,
            ArtifactMeta, ArtifactWrite, PutArtifactError, RetentionPolicy, RetentionSweepReport,
            Turn, TurnInput, TurnReceipt,
        };

        struct FakeStore {
            calls: std::sync::Mutex<Vec<String>>,
        }
        #[async_trait::async_trait]
        impl AgentSessionStore for FakeStore {
            async fn create(
                &self,
                _id: AgentSessionId,
                _kind: &str,
                _owner: &str,
                _metadata: serde_json::Value,
            ) -> AgentSessionResult<()> {
                Ok(())
            }
            async fn get(&self, _id: AgentSessionId) -> AgentSessionResult<Option<AgentSession>> {
                Ok(None)
            }
            async fn delete(&self, _id: AgentSessionId) -> AgentSessionResult<()> {
                Ok(())
            }
            async fn append_turn_with_artifacts(
                &self,
                _id: AgentSessionId,
                _turn: TurnInput,
                _artifacts: &[ArtifactWrite],
            ) -> AgentSessionResult<TurnReceipt> {
                Ok(TurnReceipt::new(1, vec![]))
            }
            async fn put_artifact_direct(
                &self,
                _id: AgentSessionId,
                _key: &str,
                _value: serde_json::Value,
                _expected_prev_version: Option<u32>,
            ) -> Result<u32, PutArtifactError> {
                Ok(1)
            }
            async fn list_turns(
                &self,
                _id: AgentSessionId,
                _since_seq: Option<u32>,
                _limit: Option<u32>,
            ) -> AgentSessionResult<Vec<Turn>> {
                Ok(vec![])
            }
            async fn latest_artifact(
                &self,
                _id: AgentSessionId,
                _key: &str,
            ) -> AgentSessionResult<Option<Artifact>> {
                Ok(None)
            }
            async fn artifact_at(
                &self,
                _id: AgentSessionId,
                _key: &str,
                _version: u32,
            ) -> AgentSessionResult<Option<Artifact>> {
                Ok(None)
            }
            async fn list_artifact_versions(
                &self,
                _id: AgentSessionId,
                _key: &str,
            ) -> AgentSessionResult<Vec<ArtifactMeta>> {
                Ok(vec![])
            }
            async fn sweep_retention(
                &self,
                kind: &str,
                _policy: &RetentionPolicy,
                _now: chrono::DateTime<chrono::Utc>,
            ) -> AgentSessionResult<RetentionSweepReport> {
                self.calls.lock().unwrap().push(kind.to_owned());
                Ok(RetentionSweepReport {
                    sessions_deleted: 1,
                    turns_deleted: 2,
                    artifacts_deleted: 3,
                })
            }
        }

        let fake = Arc::new(FakeStore {
            calls: std::sync::Mutex::new(Vec::new()),
        });
        let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let engine = Engine::new(store)
            .with_agent_session_store(
                fake.clone() as Arc<dyn starter_flow_spi::agent_session::AgentSessionStore>
            )
            .with_agent_session_retention(
                "page-builder",
                RetentionPolicy::DeleteAfter {
                    ttl: chrono::Duration::hours(1),
                },
            )
            .with_agent_session_retention(
                "chat",
                RetentionPolicy::DeleteTurnsAfter {
                    ttl: chrono::Duration::days(7),
                    keep_latest_artifact: false,
                },
            );

        let report = engine
            .sweep_agent_session_retention(chrono::Utc::now())
            .await
            .unwrap()
            .expect("two policies configured");
        assert_eq!(report.sessions_deleted, 2);
        assert_eq!(report.turns_deleted, 4);
        assert_eq!(report.artifacts_deleted, 6);

        let mut calls = fake.calls.lock().unwrap().clone();
        calls.sort();
        assert_eq!(calls, vec!["chat".to_owned(), "page-builder".to_owned()]);
    }
}
