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
            health: HealthHandle::new(),
            ai_runners: None,
            skill_selector: Arc::new(starter_flow_spi::skill::NullSkillSelector),
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

    /// Borrow the attached [`SpiRunStore`] if any. Callers wiring
    /// per-run constructors (e.g. [`crate::run::FlowRunner`]) clone
    /// this `Arc` into the run.
    pub fn run_store(&self) -> Option<&Arc<dyn SpiRunStore>> {
        self.run_store.as_ref()
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
    pub async fn start(&self) -> Result<(), EngineError> {
        self.transition(EngineState::Running)
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
}
