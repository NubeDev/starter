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

    /// Read the current value of a slot.
    async fn read_slot(&self, slot: &SlotRef) -> Result<SlotValue, GraphError>;

    /// Subscribe to slot-change events. The stream's element type is
    /// engine-internal in Phase 1; concrete `GraphEvent` lands in the
    /// engine crate alongside the propagator. The associated type
    /// keeps the trait usable without committing the event shape.
    fn subscribe(&self, opts: SubscribeOpts) -> SubscriptionStream;
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
}

impl WriteSlotOpts {
    /// Default options for a live write (`replay = false`, `force = false`).
    pub fn live() -> Self {
        Self {
            replay: false,
            force: false,
        }
    }

    /// Options for a replay write (`replay = true`, `force = false`).
    pub fn replay() -> Self {
        Self {
            replay: true,
            force: false,
        }
    }

    /// Options for a forced live write — defeats the R3 idempotent
    /// short-circuit so the write always emits a `SlotChanged` event
    /// (still subject to the [`Self::replay`] rule).
    pub fn forced() -> Self {
        Self {
            replay: false,
            force: true,
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
