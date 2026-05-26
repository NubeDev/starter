//! Graph-level contracts.
//!
//! Per `DOCS/flow/scope/SCOPE.md` R2 (slots are the only I/O surface;
//! one write chokepoint): every write to a slot — from any source —
//! enters through [`GraphStore::write_slot`]. The propagator subscribes
//! to slot changes from that single call.

use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

use crate::node::{SlotRef, SlotValue};

/// The single write chokepoint and slot-change subscription seam.
///
/// SCOPE R2: "with one write path, every invariant the engine cares
/// about (authorisation, audit, type checking, safe-state,
/// observability, replay) is enforced in one place." Any code that
/// writes a slot — REST adapter, CLI, internal propagator tick,
/// replay from checkpoint — calls [`Self::write_slot`]. Period.
#[async_trait]
pub trait GraphStore: Send + Sync + 'static {
    /// Write a value to a slot.
    ///
    /// Honours [`WriteSlotOpts::replay`]: when `true`, the
    /// implementation must reconstruct state without emitting
    /// `SlotChanged` to subscribers (R2 replay rule).
    async fn write_slot(
        &self,
        slot: &SlotRef,
        value: SlotValue,
        opts: WriteSlotOpts,
    ) -> Result<(), GraphError>;

    /// Write a batch of slot values atomically with coalesced wakes.
    ///
    /// Semantically equivalent to calling [`Self::write_slot`] in
    /// sequence for each entry, with one critical difference around
    /// event coalescing:
    ///
    /// - Writes to the **same `(node, slot)`** within one batch
    ///   each emit their own `SlotChanged` event — back-to-back
    ///   edits to one slot are distinct value-over-time changes,
    ///   matching what callers see from sequential `write_slot`
    ///   calls.
    /// - Writes to **different slots of the same node** within one
    ///   batch coalesce to a single carrier event. The motivating
    ///   case is a surface seed adapter populating several input
    ///   slots of one node on a single fire (e.g. a tool-call
    ///   node's `tool_id` + `input` seeded from YAML config).
    ///   Without coalescing, the propagator would wake that node
    ///   once per slot.
    ///
    /// Per-entry semantics still honour [`WriteSlotOpts::replay`]
    /// (no event) and [`WriteSlotOpts::force`] / R3 idempotent
    /// short-circuit (no event when value is unchanged and `force =
    /// false`); a write that's suppressed by either rule
    /// contributes no carrier. The carrier slot for the coalesced
    /// event is the first non-suppressed write to that node —
    /// subscribers must not rely on the specific carrier slot, only
    /// on the wake itself, since the propagator's input-map
    /// assembly reads every declared trigger slot from the store.
    ///
    /// Default implementation falls back to per-entry `write_slot`
    /// for backends that don't need coalescing; storage backends
    /// that share a broadcast channel (in-memory, Postgres-listen,
    /// etc.) should override to actually dedupe.
    async fn write_slot_batch(
        &self,
        writes: Vec<(SlotRef, SlotValue, WriteSlotOpts)>,
    ) -> Result<(), GraphError> {
        for (slot, value, opts) in writes {
            self.write_slot(&slot, value, opts).await?;
        }
        Ok(())
    }

    /// Read the current value of a slot.
    async fn read_slot(&self, slot: &SlotRef) -> Result<SlotValue, GraphError>;

    /// Subscribe to slot-change events. The stream's element type is
    /// engine-internal in Phase 1; concrete `GraphEvent` lands in the
    /// engine crate alongside the propagator. The associated type
    /// keeps the trait usable without committing the event shape.
    fn subscribe(&self, opts: SubscribeOpts) -> SubscriptionStream;
}

/// Provenance of a single write — stamped on the per-write `write_slot`
/// tracing span as the `origin` field so audit / replay tooling can
/// distinguish run-driven writes from definition-driven writes from
/// replay-driven writes without re-deriving it from the call site.
///
/// Per `DOCS/flow/scope/hot-reload.md` HR3: the settings path of a
/// publish writes through this same chokepoint but stamps
/// [`Self::Definition`] so the audit story tells you *why* a slot
/// moved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WriteOrigin {
    /// A normal in-flow write driven by a [`crate::node::NodeBehavior`]
    /// firing through the propagator. Default.
    #[default]
    Live,
    /// A replay write produced by the resume-from-checkpoint path
    /// (R2 replay rule). Implies `replay = true` on the
    /// [`WriteSlotOpts`] carrying this origin — set both rather
    /// than just the flag if you want the audit row to reflect it.
    Replay,
    /// A write produced by the HR1 publish chokepoint applying a
    /// settings delta to the live [`crate::graph::GraphStore`].
    /// Implies `force = false` (R2 idempotent short-circuit is
    /// honoured — a settings edit that re-asserts the current value
    /// is a no-op) and `replay = false` (the propagator must see
    /// the change so reactive downstream nodes re-tick).
    Definition,
}

impl WriteOrigin {
    /// Short stable lowercase tag suitable for the `origin` field of
    /// the per-write `write_slot` tracing span and audit columns.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Replay => "replay",
            Self::Definition => "definition",
        }
    }
}

/// Options on a single write.
///
/// SCOPE R2 replay rule: `replay = true` reconstructs `GraphStore` state
/// without re-firing subscribers, so resumed runs do not re-invoke
/// downstream side-effecting tools. `replay = false` is the live path
/// where the propagator must observe the change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WriteSlotOpts {
    /// `true` if this write is part of a replay (run resume from
    /// checkpoint, audit replay). The propagator must not emit a
    /// `SlotChanged` event for replay writes.
    pub replay: bool,
    /// `true` to defeat the R3 idempotent-write short-circuit. When
    /// `false` (the default) a `write_slot` whose new value equals the
    /// prior value emits no `SlotChanged` event and does not enqueue
    /// downstream invocations. When `true`, the write always counts as
    /// a change (subject to the [`Self::replay`] rule above).
    pub force: bool,
    /// Provenance tag stamped on the `write_slot` tracing span's
    /// `origin` field. Defaults to [`WriteOrigin::Live`].
    pub origin: WriteOrigin,
}

impl WriteSlotOpts {
    /// Default options for a live write (`replay = false`, `force = false`,
    /// `origin = Live`).
    pub fn live() -> Self {
        Self {
            replay: false,
            force: false,
            origin: WriteOrigin::Live,
        }
    }

    /// Options for a replay write (`replay = true`, `force = false`,
    /// `origin = Replay`).
    pub fn replay() -> Self {
        Self {
            replay: true,
            force: false,
            origin: WriteOrigin::Replay,
        }
    }

    /// Options for a forced live write — defeats the R3 idempotent
    /// short-circuit so the write always emits a `SlotChanged` event
    /// (still subject to the [`Self::replay`] rule).
    pub fn forced() -> Self {
        Self {
            replay: false,
            force: true,
            origin: WriteOrigin::Live,
        }
    }

    /// Options for a definition-origin write — the HR-2 settings
    /// path uses this when projecting a settings-only edit onto its
    /// config slots.
    ///
    /// `replay = false` so reactive subscribers re-tick; `force = false`
    /// so the R3 idempotent short-circuit still drops writes whose
    /// value already matches the live store; `origin = Definition`
    /// so the per-write tracing span carries `origin = "definition"`
    /// per `DOCS/flow/scope/hot-reload.md` HR3.
    pub fn config() -> Self {
        Self {
            replay: false,
            force: false,
            origin: WriteOrigin::Definition,
        }
    }
}

/// Options on a subscription.
///
/// Phase 1 is a placeholder. Concrete filtering (subscribe to a single
/// slot vs. all slots on a node vs. all slots in a flow) lands in
/// Phase 2 alongside the propagator.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct SubscribeOpts {}

/// Boxed stream of graph events. Concrete event payload lands in the
/// engine crate; the trait commits only to the stream shape.
pub type SubscriptionStream = BoxStream<'static, GraphEventEnvelope>;

/// Engine-emitted event envelope. The contents (`GraphEvent::SlotChanged`,
/// future variants) live in the engine crate to avoid committing the
/// concrete payload from the contracts crate.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct GraphEventEnvelope {
    /// The slot the event pertains to.
    pub slot: SlotRef,
    /// The new value, if the event carries one.
    pub value: Option<SlotValue>,
}

impl GraphEventEnvelope {
    /// Construct an envelope for a value-carrying event.
    pub fn slot_changed(slot: SlotRef, value: SlotValue) -> Self {
        Self {
            slot,
            value: Some(value),
        }
    }
}

/// Errors a [`GraphStore`] may return.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GraphError {
    /// The named slot is not known to the store.
    #[error("unknown slot: {0:?}")]
    UnknownSlot(SlotRef),

    /// The slot type did not match the declared kind metadata.
    #[error("type mismatch on {0:?}: {1}")]
    TypeMismatch(SlotRef, String),

    /// Backing store I/O failure.
    #[error("graph store backend failure: {0}")]
    Backend(String),
}
