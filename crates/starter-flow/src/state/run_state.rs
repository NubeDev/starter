//! Engine-typed `RunState` per R6 — the simplification that dissolved
//! the adk-rust checkpoint blob.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / "What lands in
//! `starter-flow`" — the strongly-typed, engine-owned run state that
//! replaces the opaque checkpoint payload from adk-rust. Serialized
//! by [`crate::run`] on Pause / Stopped; never exposed across the SPI
//! seam.
//!
//! Phase 2 stage 7 ships the *shape* and the in-memory accessors the
//! [`crate::run::FlowRunner`] mutates as a run progresses; persistence
//! to the SQLite `RunStore` lands in Phase 3 (see SCOPE phasing block).

use std::sync::Arc;

use tokio::sync::broadcast;

use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId, RunId};
use starter_flow_spi::node::SlotRef;

use crate::run::{RunCancel, SkillSelection};

/// Terminal-or-not status of a run.
///
/// `#[non_exhaustive]` — Phase 7 ("three-level stop") will add at least
/// `Paused` here; keeping the enum open avoids retro-breaking callers
/// that pattern-match.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunStatus {
    /// Run constructed but not yet running.
    Pending,
    /// Run is in flight — the propagator task is live.
    Running,
    /// Run finished normally (`FlowEvent::RunCompleted` emitted).
    Completed,
    /// Run failed (`FlowEvent::RunFailed` emitted). String form of the
    /// underlying [`starter_flow_spi::flow::FlowError`] is captured
    /// here for the same reason `FlowEvent::RunFailed.error` is a
    /// `String` — the variant is allowed to evolve without bumping
    /// the `RunState` shape.
    Failed(String),
    /// Run was cancelled (`FlowEvent::RunCancelled` emitted).
    Cancelled,
}

impl RunStatus {
    /// `true` if the run has reached a terminal status.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Completed | RunStatus::Failed(_) | RunStatus::Cancelled
        )
    }
}

/// Per-run engine-typed state.
///
/// SCOPE R6: this is the engine-typed replacement for adk-rust's
/// opaque checkpoint blob. Fields are deliberately concrete:
///
/// - [`Self::run`] — the run id (immutable for the lifetime of the
///   state).
/// - [`Self::flow`] / [`Self::flow_revision`] — the flow + revision
///   the run is bound to. Revisions are immutable per SCOPE
///   "Decisions made", so the revision id pins the topology.
/// - [`Self::epoch`] — monotonic per-run epoch counter the propagator
///   bumps on every hop; the in-memory snapshot here mirrors the
///   live counter for `RunStore` persistence in Phase 3.
/// - [`Self::in_flight_writes`] — count of writes the propagator has
///   issued but whose `SlotChanged` it has not yet consumed (the R2
///   chokepoint accounting).
/// - [`Self::queue_snapshot`] — the propagator's pending-work queue at
///   the last checkpoint. Phase 7 reads this back to resume a run
///   from disk; Phase 2 keeps an empty `Vec` until the propagator
///   gains an explicit pending-queue surface.
/// - [`Self::events_tx`] — the [`broadcast::Sender`] every consumer
///   subscribes to for this run (R13). Cardinality is one
///   `broadcast::Sender` per `RunId`, locked at Phase 2 stage 1.
/// - [`Self::cancel`] — the per-run [`RunCancel`] handle (R13).
/// - [`Self::skill_selection`] — the [`SkillSelection`] produced by
///   the outer-run `SkillSelector` hook (R7's outer-run binding rule;
///   set even though the `ai-agent` body lands in Phase 4 — the seam
///   exists so Phase 4 does not have to retro-fit).
/// - [`Self::status`] — the run's terminal-or-not status.
///
/// `#[non_exhaustive]` so Phase 3 / Phase 7 can add fields (persisted
/// node-state blobs, checkpoint timestamps, `Paused` accounting)
/// without bumping the shape.
#[non_exhaustive]
pub struct RunState {
    /// The run id.
    pub run: RunId,
    /// The flow this run executes.
    pub flow: FlowId,
    /// The immutable revision of the flow this run is bound to.
    pub flow_revision: FlowRevisionId,
    /// Per-run epoch counter; the propagator bumps this on every
    /// `SlotChanged` it consumes.
    pub epoch: u64,
    /// In-flight `write_slot` count (writes issued but whose
    /// downstream propagation has not yet completed).
    pub in_flight_writes: u64,
    /// Snapshot of the propagator's pending-work queue. Phase 2 keeps
    /// this as an empty `Vec`; Phase 7 fills it on checkpoint.
    pub queue_snapshot: Vec<SlotRef>,
    /// Broadcast sender every consumer of this run's events
    /// subscribes to (R13).
    pub events_tx: broadcast::Sender<FlowEvent>,
    /// Per-run cancel handle (R13).
    pub cancel: Arc<RunCancel>,
    /// Skill selection produced by the outer-run hook (R7).
    pub skill_selection: Option<Arc<SkillSelection>>,
    /// Terminal-or-not status.
    pub status: RunStatus,
}

impl RunState {
    /// Construct a freshly-pending [`RunState`].
    pub fn new(
        run: RunId,
        flow: FlowId,
        flow_revision: FlowRevisionId,
        events_tx: broadcast::Sender<FlowEvent>,
        cancel: Arc<RunCancel>,
        skill_selection: Option<Arc<SkillSelection>>,
    ) -> Self {
        Self {
            run,
            flow,
            flow_revision,
            epoch: 0,
            in_flight_writes: 0,
            queue_snapshot: Vec::new(),
            events_tx,
            cancel,
            skill_selection,
            status: RunStatus::Pending,
        }
    }
}

impl std::fmt::Debug for RunState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunState")
            .field("run", &self.run)
            .field("flow", &self.flow)
            .field("flow_revision", &self.flow_revision)
            .field("epoch", &self.epoch)
            .field("in_flight_writes", &self.in_flight_writes)
            .field("queue_snapshot", &self.queue_snapshot)
            .field("events_tx_receiver_count", &self.events_tx.receiver_count())
            .field("skill_selection.is_some", &self.skill_selection.is_some())
            .field("status", &self.status)
            .finish()
    }
}
