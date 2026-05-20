//! Flow-level contracts: ids, the run event stream, store seams.
//!
//! Per `DOCS/flow/scope/SCOPE.md` R6 (sessions persist; runs persist;
//! checkpoints are engine-typed) and R8 (flows surface as Tools and as
//! Services). The `FlowStore` / `RunStore` traits are deliberately
//! empty seams in Phase 1 — CRUD method shapes are documented but not
//! required yet; Phase 3 fleshes them out alongside the
//! `starter-store-sqlite` implementations.

use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::node::{IdError, NodeError, NodeId, SlotMap, SlotRef, SlotValue};
use crate::Principal;

/// Reverse-DNS flow identifier (SCOPE R10). Same validation rules as
/// [`NodeId`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FlowId(String);

impl FlowId {
    /// Parse a string as a flow id.
    pub fn new(s: impl Into<String>) -> Result<Self, IdError> {
        let s = s.into();
        crate::node::validate_reverse_dns(&s)?;
        Ok(Self(s))
    }

    /// Borrow the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FlowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for FlowId {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FlowId> for String {
    fn from(value: FlowId) -> Self {
        value.0
    }
}

/// UUID-backed identifier for a specific revision of a flow.
///
/// SCOPE "Decisions made": "revisions are immutable; `head_seq` pointer
/// per flow tracks the current revision". The revision id is the
/// immutable handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FlowRevisionId(pub Uuid);

impl FlowRevisionId {
    /// Generate a fresh revision id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FlowRevisionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FlowRevisionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// UUID-backed identifier for a single flow run (one invocation, start
/// to terminal state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(pub Uuid);

impl RunId {
    /// Generate a fresh run id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Events streamed for the lifetime of a flow run.
///
/// SCOPE R13: `Stream<FlowEvent>` — same shape `starter-ai`'s `OnEvent`
/// and `starter_spi`'s event channels already use. Adapters render
/// natively per transport (SSE, NDJSON, MCP `notifications/progress`,
/// gRPC server-streaming).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlowEvent {
    /// Run started. Emitted once at the top of every run.
    RunStarted {
        /// The run id.
        run: RunId,
        /// The flow being run.
        flow: FlowId,
    },
    /// A node started executing.
    NodeStarted {
        /// The run id.
        run: RunId,
        /// The node that started.
        node: NodeId,
    },
    /// A node emitted a value on one of its output slots.
    NodeEmitted {
        /// The run id.
        run: RunId,
        /// The node that emitted.
        node: NodeId,
        /// The output slot name on that node.
        slot: String,
        /// The value written.
        value: SlotValue,
    },
    /// A node failed.
    NodeFailed {
        /// The run id.
        run: RunId,
        /// The node that failed.
        node: NodeId,
        /// String form of the underlying [`NodeError`]. Kept as a
        /// string here so [`FlowEvent`] stays `Serialize` regardless
        /// of the concrete error variant a kind returns.
        error: String,
    },
    /// Run completed normally. Output is the terminal-node output
    /// map (per R9, this is the value `FlowAsTool` returns).
    RunCompleted {
        /// The run id.
        run: RunId,
        /// The terminal output map.
        output: SlotMap,
    },
    /// Run failed.
    RunFailed {
        /// The run id.
        run: RunId,
        /// String form of the underlying [`FlowError`].
        error: String,
    },
    /// Run was cancelled via its [`Cancel`](crate::Cancel) token.
    RunCancelled {
        /// The run id.
        run: RunId,
    },
    /// A `RunStore::checkpoint` (or `RunStore::finish`) call failed
    /// and the engine is retrying with exponential backoff. Emitted
    /// once per attempt, with `attempt` running `1..=5`. After the
    /// 5th the engine transitions to
    /// [`EngineHealth::Degraded`]; see D-F3.11 in the Phase 3 job
    /// SCOPE.
    CheckpointFailed {
        /// The run whose checkpoint failed.
        run: RunId,
        /// The underlying [`FlowError`], stringified for portability
        /// across the per-run broadcast.
        error: String,
        /// 1-indexed retry attempt counter.
        attempt: u32,
    },
    /// `FlowAsService` detected a re-delivered event whose dedup key
    /// already names a prior run; the engine short-circuited to that
    /// prior run's outcome rather than starting a new run. See
    /// D-F3.12 in the Phase 3 job SCOPE.
    DedupShortCircuit {
        /// The prior run whose outcome the service returned.
        prior_run_id: RunId,
    },
}

impl FlowEvent {
    /// Convenience constructor for [`FlowEvent::NodeFailed`].
    pub fn node_failed(run: RunId, node: NodeId, err: &NodeError) -> Self {
        Self::NodeFailed {
            run,
            node,
            error: err.to_string(),
        }
    }

    /// Convenience constructor for [`FlowEvent::RunFailed`].
    pub fn run_failed(run: RunId, err: &FlowError) -> Self {
        Self::RunFailed {
            run,
            error: err.to_string(),
        }
    }
}

/// Errors that fail an entire run, or that a store seam surfaces to
/// its caller.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FlowError {
    /// Per-run cycle budget exhausted (R1).
    #[error("cycle budget exhausted after {hops} propagation hops")]
    CycleBudgetExhausted {
        /// Hop count at which the cap fired.
        hops: u64,
    },
    /// A node returned a [`NodeError`] under an `on_failure: abort`
    /// policy (R3).
    #[error("node {node} failed: {error}")]
    NodeAborted {
        /// The aborting node.
        node: NodeId,
        /// The underlying node error, stringified for portability.
        error: String,
    },
    /// A store seam looked for a record and found none. Surfaced by
    /// e.g. [`FlowStore::load`] / [`RunStore::load`] /
    /// [`SessionStore::get`] when a required id is absent. The
    /// `kind` is a short discriminator (`"flow"`, `"run"`, etc.) and
    /// `id` is the stringified id the caller asked for, so the
    /// message is actionable without a log-line attached.
    #[error("{kind} not found: {id}")]
    NotFound {
        /// Short discriminator naming the missing resource kind.
        kind: &'static str,
        /// Stringified id the caller passed in.
        id: String,
    },
    /// Run-store / flow-store backend failure.
    #[error("flow backend failure: {0}")]
    Backend(String),
}

/// Result alias for store-seam methods.
pub type FlowResult<T> = std::result::Result<T, FlowError>;

/// Engine-level health surface.
///
/// Per D-F3.11 in the Phase 3 job SCOPE. Pulled (not pushed):
/// `Engine::health()` is a sync, lock-free accessor over an
/// `AtomicU8`. A future job may add a periodic
/// `FlowEvent::HealthChanged` once a Phase-7-owned engine-level
/// event bus exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum EngineHealth {
    /// Engine is serving normally; checkpoints are persisting on the
    /// happy path.
    Healthy,
    /// Engine is serving in-flight runs from in-memory state because
    /// the `RunStore` backend has returned 5 consecutive checkpoint
    /// failures. [`Engine::start`](crate) (the engine-side API)
    /// rejects new runs with
    /// [`EngineError::BackendUnavailable`] while degraded.
    /// In-memory checkpoint queues drain in `(run_id, seq)` order
    /// on the next successful checkpoint write and the engine
    /// transitions back to [`Self::Healthy`].
    Degraded,
}

impl fmt::Display for EngineHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
        })
    }
}

/// Engine-level errors raised by the Phase 3 SPI surface.
///
/// Distinct from the engine crate's internal state-machine
/// `EngineError` (which carries `IllegalTransition` variants for
/// the R12 transition matrix). This SPI-level type names the
/// errors that cross the store/engine boundary and surface to
/// engine API callers. Per D-F3.11 in the Phase 3 job SCOPE,
/// `BackendUnavailable` is what `Engine::start` returns while the
/// engine is in [`EngineHealth::Degraded`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EngineError {
    /// The `RunStore` backend is unreachable; the engine is
    /// degraded and refusing new runs while it drains its
    /// in-memory checkpoint queue. Transient — retry once the
    /// engine returns to [`EngineHealth::Healthy`].
    #[error("engine backend unavailable")]
    BackendUnavailable,
    /// A store seam call failed during a public engine API call.
    /// Wraps the underlying [`FlowError`].
    #[error(transparent)]
    Flow(#[from] FlowError),
}

/// Per-run observability counters.
///
/// Pulled via `Engine::run_metrics(run_id) -> Option<RunMetrics>`.
/// Per D-F3.10 and D-F3.11 in the Phase 3 job SCOPE. The struct is
/// `#[non_exhaustive]` so future counters land additively.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunMetrics {
    /// Total number of [`FlowEvent`]s a per-run broadcast subscriber
    /// has been told it missed via `tokio::sync::broadcast`'s
    /// `RecvError::Lagged(n)`. Non-zero only when a subscriber
    /// consumes slower than the producer fills
    /// [`RunOpts::event_broadcast_capacity`].
    pub subscriber_lagged_count: u64,
    /// Total number of checkpoint batches dropped by the engine's
    /// degraded-mode in-memory queue when it exceeded
    /// [`RunOpts::degraded_queue_capacity`]. Evict-oldest policy
    /// per D-F3.11; non-zero only when the engine has been
    /// [`EngineHealth::Degraded`] long enough to fill the queue.
    pub degraded_dropped_count: u64,
}

impl RunMetrics {
    /// Construct an empty [`RunMetrics`] (both counters zero).
    pub fn zero() -> Self {
        Self::default()
    }
}

/// Checkpoint-history retention policy for a single run.
///
/// Per D-F3.9 in the Phase 3 job SCOPE. Default is
/// [`Self::Bounded`]`(100)`: keep the last 100 checkpoints per
/// open run plus the final checkpoint per finished run. Pruning
/// runs inside the same SQLite transaction as the checkpoint
/// insert, so the table never momentarily exceeds the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum CheckpointRetention {
    /// Keep the last `n` checkpoints per open run plus the final
    /// row per finished run.
    Bounded(usize),
    /// Keep every checkpoint; never prune. Reserved for
    /// audit-regulated deployments that need full slot-write
    /// history.
    Unbounded,
}

impl Default for CheckpointRetention {
    fn default() -> Self {
        Self::Bounded(100)
    }
}

/// Per-run engine knobs.
///
/// Phase 2 carried these as `PropagatorConfig` fields inside the
/// engine crate; Phase 3 lifts them into the SPI so the
/// `Engine::with_run_store(…)` builder hook (stage 5) can accept
/// per-run policy without depending on the engine-internal config
/// type. Stage 5 projects [`Self::max_propagation_hops`] into the
/// existing `PropagatorConfig` — the propagator's own API is
/// unchanged.
///
/// `#[non_exhaustive]`; future fields (e.g. `checkpoint_backoff`,
/// `on_backend_failure`) absorb additively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunOpts {
    /// R1 cycle-bound budget. Each `SlotChanged` propagation
    /// schedule increments a per-run hop counter; exceeding this
    /// marks the run [`FlowError::CycleBudgetExhausted`].
    pub max_propagation_hops: u64,
    /// When `true` (the default), a `write_slot` whose new value
    /// equals the prior value short-circuits — no `SlotChanged`
    /// event, no downstream invocation. D1a in the Phase 2
    /// SCOPE.
    pub idempotent_short_circuit: bool,
    /// Checkpoint-history retention policy (D-F3.9).
    pub checkpoint_retention: CheckpointRetention,
    /// Capacity of the per-run
    /// `tokio::sync::broadcast::Sender<FlowEvent>`. Producers never
    /// block; slow consumers see `RecvError::Lagged(n)` on next
    /// `recv` (D-F3.10).
    pub event_broadcast_capacity: usize,
    /// Capacity of the per-run in-memory checkpoint-batch queue
    /// the engine keeps while [`EngineHealth::Degraded`]
    /// (D-F3.11). Evict-oldest on overflow;
    /// [`RunMetrics::degraded_dropped_count`] tracks loss.
    pub degraded_queue_capacity: usize,
}

impl Default for RunOpts {
    fn default() -> Self {
        Self {
            // Matches the engine crate's existing
            // `DEFAULT_MAX_PROPAGATION_HOPS = 1000`.
            max_propagation_hops: 1_000,
            idempotent_short_circuit: true,
            checkpoint_retention: CheckpointRetention::default(),
            event_broadcast_capacity: 1_024,
            degraded_queue_capacity: 1_024,
        }
    }
}

/// Engine-typed run state, written into every checkpoint.
///
/// SCOPE R6: "checkpoints are engine-typed". The state machine
/// here is the subset the propagator + the resume path need; the
/// engine's richer internal state (Phase 2's
/// `Starting → Running → Pausing → … → Stopped` lifecycle) is
/// **engine-global**, not per-run, and stays in the engine crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    /// Run is actively ticking; the propagator owns it.
    Running,
    /// Run is paused via per-flow pause (Phase 7 will exercise
    /// this; included in the enum now so the checkpoint schema
    /// doesn't need a migration when Phase 7 lands).
    Paused,
    /// Run completed normally; the terminal output slot value is
    /// the run's outcome.
    Completed,
    /// Run failed with a [`FlowError`]; the failure is stringified
    /// in the run record.
    Failed,
    /// Run was cancelled via its [`Cancel`](crate::Cancel) token.
    Cancelled,
}

/// Terminal-state record written by `RunStore::finish`.
///
/// Mirrors the [`FlowEvent::RunCompleted`] /
/// [`FlowEvent::RunFailed`] / [`FlowEvent::RunCancelled`]
/// variants but lives durably; the broadcast is for live
/// observers, the store is for durable record (R6 + D-F3.10
/// observe-vs-record split).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunOutcome {
    /// Run finished normally; the terminal output map is what
    /// `FlowAsTool::call` returns and what the run record
    /// stores.
    Completed {
        /// The terminal output map.
        output: SlotMap,
    },
    /// Run failed. Carries the stringified [`FlowError`] for
    /// portability across the store boundary.
    Failed {
        /// Stringified failure.
        error: String,
    },
    /// Run was cancelled.
    Cancelled,
}

/// One immutable revision of a flow definition.
///
/// SCOPE "Decisions made": "revisions are immutable; `head_seq`
/// pointer per flow tracks the current revision". `body` is
/// kept as `serde_json::Value` so the flow-definition schema
/// can evolve in the engine without an SPI bump.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FlowRevision {
    /// The flow this revision belongs to.
    pub flow_id: FlowId,
    /// This revision's id.
    pub revision_id: FlowRevisionId,
    /// Opaque flow definition body. Phase 5's richer typing will
    /// crystallise a concrete struct here; until then the
    /// engine + UI co-own the JSON schema.
    pub body: serde_json::Value,
    /// Provenance tag from the hot-reload publish chokepoint
    /// (`DefinitionSource::audit_tag` in `starter-flow`).
    /// Persisted on the `flow_revisions` table per
    /// `DOCS/flow/scope/hot-reload.md` HR3 (Replay/audit). Stored
    /// as an opaque short string so the engine can evolve the
    /// `DefinitionSource` enum without the store needing to know;
    /// shapes like `"api"`, `"cli"`, `"file:/etc/flows/foo.json"`,
    /// `"extension:com.example.tools"` are the contract today.
    ///
    /// Defaults to `"api"` so existing callers that construct a
    /// `FlowRevision` directly do not need to be retrofitted; the
    /// hot-reload chokepoint stamps the real source via
    /// [`Self::with_source`].
    #[serde(default = "FlowRevision::default_source")]
    pub source: String,
}

impl FlowRevision {
    /// Construct a [`FlowRevision`] with the default
    /// [`Self::source`] tag (`"api"`).
    pub fn new(flow_id: FlowId, revision_id: FlowRevisionId, body: serde_json::Value) -> Self {
        Self {
            flow_id,
            revision_id,
            body,
            source: Self::default_source(),
        }
    }

    /// Builder-style override of [`Self::source`]. The hot-reload
    /// publish chokepoint calls this with
    /// `DefinitionSource::audit_tag` (see `starter-flow`).
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    fn default_source() -> String {
        "api".to_string()
    }
}

/// One atomic per-tick checkpoint, written by `RunStore::checkpoint`.
///
/// `seq` is the propagator's tick counter at the moment the batch
/// was committed (D-F3.2 + Q2). `writes` is the batch of slot
/// writes that occurred during that tick — the resume path
/// replays them through the single `GraphStore::write_slot`
/// chokepoint (R2 unchanged).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunCheckpoint {
    /// The run this checkpoint belongs to.
    pub run_id: RunId,
    /// Per-run monotonic tick counter; the `(run_id, seq)` pair
    /// is the primary key in `run_checkpoints`.
    pub seq: u64,
    /// Engine-typed state at the moment the checkpoint was
    /// committed.
    pub state: RunState,
    /// Slot writes that produced this revision of the run state.
    pub writes: Vec<(SlotRef, SlotValue)>,
}

impl RunCheckpoint {
    /// Construct a [`RunCheckpoint`].
    pub fn new(
        run_id: RunId,
        seq: u64,
        state: RunState,
        writes: Vec<(SlotRef, SlotValue)>,
    ) -> Self {
        Self {
            run_id,
            seq,
            state,
            writes,
        }
    }
}

/// UUID-backed session identifier.
///
/// Sessions persist (R6) and group runs that share LLM context /
/// principal / tenant boundary. The exact `session_policy`
/// semantics (`fresh | continue | long-lived`) are R3 concerns —
/// `SessionStore` is just the persistence seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    /// Generate a fresh session id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Derive a deterministic session id for an `ai-agent` node
    /// invocation per [`SessionMode`] (D-F4.6).
    ///
    /// - [`SessionMode::FreshPerInvocation`] returns a fresh
    ///   [`Uuid::new_v4`].
    /// - [`SessionMode::ReuseAcrossRun`] hashes `(node_id, run_id)`
    ///   into a deterministic [`Uuid::new_v5`].
    /// - [`SessionMode::ReuseAcrossFlow`] hashes
    ///   `(node_id, flow_id, principal.id())` into a deterministic
    ///   [`Uuid::new_v5`].
    pub fn for_ai_agent_node(
        mode: SessionMode,
        node_id: &crate::node::NodeId,
        run_id: RunId,
        flow_id: FlowId,
        principal: &crate::Principal,
    ) -> Self {
        match mode {
            SessionMode::FreshPerInvocation => Self::new(),
            SessionMode::ReuseAcrossRun => {
                let key = format!("{node_id}|{run_id}");
                Self(Uuid::new_v5(&SESSION_NS, key.as_bytes()))
            }
            SessionMode::ReuseAcrossFlow => {
                let key = format!("{node_id}|{flow_id}|{}", principal.subject);
                Self(Uuid::new_v5(&SESSION_NS, key.as_bytes()))
            }
        }
    }
}

/// UUID namespace for deterministic [`SessionId::for_ai_agent_node`]
/// derivations. Frozen — changing this value invalidates every
/// persisted `ReuseAcrossRun`/`ReuseAcrossFlow` session.
const SESSION_NS: Uuid = Uuid::from_bytes([
    0x6f, 0x39, 0xa1, 0xb2, 0x55, 0x4c, 0x4e, 0xa9, 0x9d, 0xe7, 0x4f, 0x21, 0x80, 0x12, 0x10, 0x73,
]);

/// Session-continuity policy for an `ai-agent` node invocation
/// (D-F4.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SessionMode {
    /// Fresh `SessionId` per invocation. Default — cheapest and most
    /// predictable.
    #[default]
    FreshPerInvocation,
    /// Reuse the same `SessionId` across every invocation of this
    /// node within one run. Keyed on `(node_id, run_id)`.
    ReuseAcrossRun,
    /// Reuse the same `SessionId` across every invocation of this
    /// node across every run of the owning flow for one principal.
    /// Keyed on `(node_id, flow_id, principal_id)`.
    ReuseAcrossFlow,
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// One persisted session record.
///
/// The `body` is opaque JSON the engine layers above SPI populate
/// (LLM message history, tool-use trace, scratchpad slots). The
/// SPI only commits to the persistence shape (id + principal +
/// body), per R6 ("sessions persist; runs persist; checkpoints
/// are engine-typed" — sessions are engine-typed too, but Phase
/// 3's job is the persistence seam, not the typing).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SessionRecord {
    /// The session this record belongs to.
    pub session_id: SessionId,
    /// Owning principal (R3 auth).
    pub principal: Principal,
    /// Opaque session body; see type-level doc.
    pub body: serde_json::Value,
}

impl SessionRecord {
    /// Construct a [`SessionRecord`].
    pub fn new(session_id: SessionId, principal: Principal, body: serde_json::Value) -> Self {
        Self {
            session_id,
            principal,
            body,
        }
    }
}

/// Persistence seam for flow definitions (R6).
///
/// Phase 3 fleshes out the CRUD method shape the SCOPE
/// "Decisions made" block named in Phase 1. All methods are
/// `async`, take `&self`, and return [`FlowResult`]; impls live
/// in `starter-store-sqlite` behind a default-off `flow`
/// feature (D-F3.3). The trait is intentionally minimal — paging,
/// principal-scoped listing, and other extensions land
/// additively in follow-up SPI bumps with their own baseline
/// regenerations (D-F3.1 revisit trigger).
#[async_trait]
pub trait FlowStore: Send + Sync + 'static {
    /// Load a specific flow revision. `revision = None` loads
    /// the flow's `head` revision (or returns
    /// [`FlowError::NotFound`] if the flow has no head).
    async fn load(
        &self,
        flow_id: FlowId,
        revision: Option<FlowRevisionId>,
    ) -> FlowResult<FlowRevision>;

    /// Persist a new immutable revision. The store assigns no
    /// revision id of its own — callers supply
    /// [`FlowRevision::revision_id`] so the engine can decide
    /// whether to treat the put as a new revision or as an
    /// idempotent re-put of an existing revision. Returns the
    /// revision id that is now durable.
    async fn put(&self, revision: FlowRevision) -> FlowResult<FlowRevisionId>;

    /// List every flow id the store knows about.
    async fn list(&self) -> FlowResult<Vec<FlowId>>;

    /// List the revisions of a single flow, newest first.
    async fn revisions(&self, flow_id: FlowId) -> FlowResult<Vec<FlowRevisionId>>;

    /// The flow's current head revision, if any.
    async fn head(&self, flow_id: FlowId) -> FlowResult<Option<FlowRevisionId>>;
}

/// Persistence seam for flow runs and per-run checkpoints (R6).
///
/// Per-tick checkpoint cadence (D-F3.2): the engine calls
/// [`Self::checkpoint`] **once per propagator tick** with the
/// batch of slot writes that occurred during the tick — never
/// once per `GraphStore::write_slot` call. The batch is passed
/// by borrow (`&[(SlotRef, SlotValue)]`) so the propagator
/// checkpoints without allocating an owned `Vec`.
///
/// `find_by_dedup_key` powers the D-F3.12 short-circuit lookup:
/// `FlowAsService` resolves a dedup key per event then queries
/// here before starting a new run; a hit returns the prior
/// run's outcome.
#[async_trait]
pub trait RunStore: Send + Sync + 'static {
    /// Record a new run as starting. The store writes the
    /// initial `runs` row; subsequent checkpoints reference
    /// this `run_id`. A service-driven run (one whose
    /// invocation is mediated by `FlowAsService`) passes a
    /// `Some(_)` `dedup` so the [`Self::find_by_dedup_key`]
    /// lookup will find it on re-delivery; non-service runs
    /// pass `None`.
    async fn start(
        &self,
        run_id: RunId,
        flow_revision: FlowRevisionId,
        opts: RunOpts,
        principal: Principal,
        dedup: Option<DedupKey>,
    ) -> FlowResult<()>;

    /// Persist one per-tick checkpoint atomically (D-F3.8) and
    /// run the in-tx pruning step (D-F3.9). `seq` is the
    /// propagator's tick counter at the moment the batch was
    /// committed — the resume path uses `MAX(seq)` to find the
    /// latest checkpoint, so the propagator's monotonic tick
    /// counter (Q2) must drive this value (the store does
    /// **not** auto-increment).
    async fn checkpoint(
        &self,
        run_id: RunId,
        seq: u64,
        state: RunState,
        writes: &[(SlotRef, SlotValue)],
    ) -> FlowResult<()>;

    /// Load the latest checkpoint for a run, or `None` if the
    /// run has no checkpoint yet (a run that crashed before its
    /// first tick committed). The resume path reads this on
    /// `Engine::start(known_run_id)` and replays
    /// [`RunCheckpoint::writes`] through
    /// `GraphStore::write_slot` (R2 unchanged: the resume path
    /// is not a second writer).
    async fn load(&self, run_id: RunId) -> FlowResult<Option<RunCheckpoint>>;

    /// Mark a run finished with its terminal outcome. Atomic
    /// with the final checkpoint preservation per D-F3.9.
    async fn finish(&self, run_id: RunId, outcome: RunOutcome) -> FlowResult<()>;

    /// List every run that has not yet been [`Self::finish`]ed.
    /// Used on engine boot to drive the resume-from-checkpoint
    /// walk for runs that were in-flight when the previous
    /// process exited.
    async fn list_open(&self) -> FlowResult<Vec<RunId>>;

    /// Look up the run id of a prior `FlowAsService` invocation
    /// for the given `(service_name, dedup_key)` pair, or
    /// `None` if no such run exists. Backed by the
    /// `UNIQUE (service_name, dedup_key)` partial index on the
    /// `runs` table (D-F3.12).
    async fn find_by_dedup_key(
        &self,
        service_name: &str,
        dedup_key: &str,
    ) -> FlowResult<Option<RunId>>;
}

/// Per-service dedup key the engine threads through
/// `RunStore::start` so future re-deliveries
/// (`RunStore::find_by_dedup_key`) can short-circuit.
///
/// Carried as a typed pair so the store impl never has to guess
/// which of two `String`s is which.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DedupKey {
    /// The `FlowAsService::service_name` the event was
    /// delivered to.
    pub service_name: String,
    /// The dedup key computed for this event per D-F3.12 —
    /// either [`EventSink::dedup_key`](starter_spi::service::EventSink::dedup_key)
    /// or the blake3 fallback.
    pub key: String,
}

impl DedupKey {
    /// Construct a [`DedupKey`].
    pub fn new(service_name: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            key: key.into(),
        }
    }
}

/// Persistence seam for sessions (R6).
///
/// Q1 locked: `SessionStore` lives in `starter-flow-spi::flow`
/// alongside [`FlowStore`] / [`RunStore`]. Moves to
/// `starter-spi` only if a non-flow consumer surfaces a need.
#[async_trait]
pub trait SessionStore: Send + Sync + 'static {
    /// Fetch a session by id, or `None` if it doesn't exist.
    async fn get(&self, session_id: SessionId) -> FlowResult<Option<SessionRecord>>;

    /// Insert-or-update a session record.
    async fn put(&self, session_id: SessionId, record: SessionRecord) -> FlowResult<()>;

    /// List every session a principal owns.
    async fn list(&self, principal: Principal) -> FlowResult<Vec<SessionId>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_opts_defaults_match_scope() {
        let o = RunOpts::default();
        assert_eq!(o.max_propagation_hops, 1_000);
        assert!(o.idempotent_short_circuit);
        assert_eq!(o.event_broadcast_capacity, 1_024);
        assert_eq!(o.degraded_queue_capacity, 1_024);
        assert_eq!(o.checkpoint_retention, CheckpointRetention::Bounded(100));
    }

    #[test]
    fn run_metrics_zero() {
        let m = RunMetrics::zero();
        assert_eq!(m.subscriber_lagged_count, 0);
        assert_eq!(m.degraded_dropped_count, 0);
    }

    #[test]
    fn engine_health_display() {
        assert_eq!(EngineHealth::Healthy.to_string(), "healthy");
        assert_eq!(EngineHealth::Degraded.to_string(), "degraded");
    }

    #[test]
    fn flow_error_notfound_message() {
        let e = FlowError::NotFound {
            kind: "flow",
            id: "com.acme.greet".to_string(),
        };
        assert_eq!(e.to_string(), "flow not found: com.acme.greet");
    }

    #[test]
    fn engine_error_from_flow_error() {
        let e: EngineError = FlowError::Backend("oops".into()).into();
        assert!(matches!(e, EngineError::Flow(FlowError::Backend(_))));
    }

    #[test]
    fn run_opts_roundtrip_json() {
        // Tolerant deserializer per Q5: additive fields absorbed.
        let o = RunOpts::default();
        let s = serde_json::to_string(&o).unwrap();
        let back: RunOpts = serde_json::from_str(&s).unwrap();
        assert_eq!(o, back);
    }

    #[test]
    fn dedup_key_pair_roundtrip() {
        let k = DedupKey::new("com.acme.svc", "abc123");
        let s = serde_json::to_string(&k).unwrap();
        let back: DedupKey = serde_json::from_str(&s).unwrap();
        assert_eq!(k, back);
    }

    #[test]
    fn checkpoint_retention_default_is_bounded_100() {
        let r = CheckpointRetention::default();
        assert!(matches!(r, CheckpointRetention::Bounded(100)));
    }

    #[test]
    fn run_state_serializes_snake_case() {
        let s = serde_json::to_string(&RunState::Running).unwrap();
        assert_eq!(s, r#""running""#);
        let s = serde_json::to_string(&RunState::Cancelled).unwrap();
        assert_eq!(s, r#""cancelled""#);
    }
}
