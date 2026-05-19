//! Graph storage + the single `write_slot` chokepoint per R2.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / "What lands in
//! `starter-flow`" — the in-engine [`GraphStore`] impl that funnels
//! every slot write through one path so the propagator can observe
//! and re-fire subscribers (with the `replay: bool` opt-out per R2,
//! and the R3 idempotent-write short-circuit overridable per-write
//! via [`WriteSlotOpts::force`]).
//!
//! Phase-2 scope (D1b from SCOPE "Decisions made"): a single
//! in-memory [`InMemoryGraphStore`] impl backed by
//! `tokio::sync::RwLock<BTreeMap<NodeId, Node>>` for storage and a
//! single per-store `tokio::sync::broadcast::Sender<GraphEvent>` for
//! the change stream. The SQLite `FlowStore` / `RunStore` /
//! `SessionStore` impls land in Phase 3 in `starter-store-sqlite` and
//! do not pre-decide their on-disk shape beyond the [`GraphStore`]
//! trait contract.

use std::collections::BTreeMap;

use async_trait::async_trait;
use futures::{stream::BoxStream, StreamExt};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;

use starter_flow_spi::graph::{
    GraphError, GraphEventEnvelope, GraphStore, SubscribeOpts, SubscriptionStream, WriteSlotOpts,
};
use starter_flow_spi::node::{NodeId, SlotRef, SlotValue};

/// One node's slot state inside the in-memory graph.
///
/// Phase 2 is intentionally minimal — a [`SlotValue`] per slot name.
/// Kind metadata (declared slots, facets, policies) is read from the
/// node-kind registry, not from this struct.
#[derive(Debug, Default, Clone)]
pub struct Node {
    /// Slot values keyed by slot name.
    slots: BTreeMap<String, SlotValue>,
}

impl Node {
    /// Construct an empty [`Node`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Borrow the slot map (test / inspector convenience — the engine
    /// itself never reads slots through this; it goes through
    /// [`GraphStore::read_slot`]).
    pub fn slots(&self) -> &BTreeMap<String, SlotValue> {
        &self.slots
    }
}

/// Engine-internal graph event carried on the per-store broadcast
/// channel.
///
/// SCOPE R2: the propagator subscribes to `SlotChanged` from one
/// chokepoint and fans values downstream along outbound `Link`s.
/// Replay writes ([`WriteSlotOpts::replay`] = `true`) do not produce
/// this event; idempotent writes (R3) do not produce this event
/// unless [`WriteSlotOpts::force`] is set.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum GraphEvent {
    /// A slot's value changed and the propagator should react.
    SlotChanged {
        /// The slot whose value changed.
        slot: SlotRef,
        /// The newly-written value.
        value: SlotValue,
    },
}

impl GraphEvent {
    /// Map an engine-internal event to the SPI subscriber envelope.
    fn into_envelope(self) -> GraphEventEnvelope {
        match self {
            Self::SlotChanged { slot, value } => GraphEventEnvelope::slot_changed(slot, value),
        }
    }
}

/// Default broadcast channel capacity for the per-store change stream.
///
/// Phase 2 picks a round number; tunable in later phases once the
/// propagator's burst characteristics are measured.
const DEFAULT_BROADCAST_CAPACITY: usize = 1024;

/// In-memory [`GraphStore`] for Phase 2.
///
/// Locked by D1b in SCOPE "Decisions made": `BTreeMap<NodeId, Node>`
/// guarded by `tokio::sync::RwLock`; subscriptions backed by a single
/// per-store `tokio::sync::broadcast::Sender`. Single writer goes
/// through [`Self::write_slot`] (R2); subscribers receive
/// [`GraphEvent::SlotChanged`] (mapped to [`GraphEventEnvelope`] on
/// the wire) from one per-store broadcast.
pub struct InMemoryGraphStore {
    nodes: RwLock<BTreeMap<NodeId, Node>>,
    tx: broadcast::Sender<GraphEvent>,
}

impl Default for InMemoryGraphStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryGraphStore {
    /// Construct an empty store with the default broadcast capacity.
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_BROADCAST_CAPACITY)
    }

    /// Construct an empty store with a caller-chosen broadcast
    /// channel capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self {
            nodes: RwLock::new(BTreeMap::new()),
            tx,
        }
    }
}

#[async_trait]
impl GraphStore for InMemoryGraphStore {
    /// The single write chokepoint per R2.
    ///
    /// Every write opens a `write_slot` span recording `node_id`,
    /// `slot_name`, `replay_flag`, `force_flag`, and `prev_was_equal`.
    /// The propagator-facing `SlotChanged` event is emitted unless:
    ///
    /// - `opts.replay == true` (R2 replay rule — replay reconstructs
    ///   state without re-firing subscribers; the engine's safe-state
    ///   drive under R12 takes a *different* code path that DOES
    ///   publish, but that path lives in the engine module not here),
    ///   OR
    /// - the new value equals the prior value and `opts.force == false`
    ///   (R3 idempotent-write short-circuit).
    async fn write_slot(
        &self,
        slot: &SlotRef,
        value: SlotValue,
        opts: WriteSlotOpts,
    ) -> Result<(), GraphError> {
        let span = tracing::info_span!(
            "write_slot",
            node_id = %slot.node,
            slot_name = %slot.slot,
            replay_flag = opts.replay,
            force_flag = opts.force,
            prev_was_equal = tracing::field::Empty,
        );
        let _enter = span.enter();

        let mut nodes = self.nodes.write().await;
        let entry = nodes.entry(slot.node.clone()).or_default();
        let prev_was_equal = entry
            .slots
            .get(&slot.slot)
            .is_some_and(|prev| prev == &value);
        span.record("prev_was_equal", prev_was_equal);

        // R3 idempotent-write short-circuit. `force = true` bypasses it.
        if prev_was_equal && !opts.force {
            return Ok(());
        }

        entry.slots.insert(slot.slot.clone(), value.clone());
        // Release the write lock before fanning out so subscribers that
        // immediately call back into the store (e.g. the propagator
        // reading downstream slots) don't deadlock.
        drop(nodes);

        // R2 replay rule: replay writes reconstruct state without
        // re-firing subscribers.
        if !opts.replay {
            // `send` only errors when there are zero active receivers;
            // a store with no subscribers is a perfectly valid state.
            let _ = self.tx.send(GraphEvent::SlotChanged {
                slot: slot.clone(),
                value,
            });
        }

        Ok(())
    }

    async fn read_slot(&self, slot: &SlotRef) -> Result<SlotValue, GraphError> {
        let nodes = self.nodes.read().await;
        let node = nodes
            .get(&slot.node)
            .ok_or_else(|| GraphError::UnknownSlot(slot.clone()))?;
        node.slots
            .get(&slot.slot)
            .cloned()
            .ok_or_else(|| GraphError::UnknownSlot(slot.clone()))
    }

    fn subscribe(&self, _opts: SubscribeOpts) -> SubscriptionStream {
        let rx = self.tx.subscribe();
        let stream = BroadcastStream::new(rx).filter_map(|item| async move {
            // `BroadcastStream` surfaces `Err(Lagged(_))` when a
            // receiver falls behind capacity. Phase-2 policy:
            // silently skip lagged batches — the propagator's
            // correctness relies on R2 single-chokepoint + R3
            // idempotency, not on every event reaching every
            // subscriber. Operator-facing surfacing of lag lives in
            // Phase 7 alongside the per-run event stream.
            item.ok().map(GraphEvent::into_envelope)
        });
        Box::pin(stream) as BoxStream<'static, GraphEventEnvelope>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    fn slot(node: &str, name: &str) -> SlotRef {
        SlotRef::new(NodeId::new(node).unwrap(), name)
    }

    /// A single live write produces exactly one `SlotChanged` event
    /// on a subscriber that subscribed first.
    #[tokio::test]
    async fn single_writer_emits_one_slot_changed() {
        let store = InMemoryGraphStore::new();
        let mut sub = store.subscribe(SubscribeOpts::default());

        let s = slot("com.acme.n1", "out");
        store
            .write_slot(&s, SlotValue::Int(42), WriteSlotOpts::live())
            .await
            .unwrap();

        let ev = timeout(Duration::from_millis(200), sub.next())
            .await
            .expect("subscriber did not see the write")
            .expect("stream ended");
        assert_eq!(ev.slot, s);
        assert_eq!(ev.value, Some(SlotValue::Int(42)));

        // No second event in flight.
        let none = timeout(Duration::from_millis(50), sub.next()).await;
        assert!(none.is_err(), "unexpected extra event: {none:?}");
    }

    /// A replay write reconstructs state without emitting any
    /// `SlotChanged` event (R2 replay rule).
    #[tokio::test]
    async fn replay_write_emits_no_slot_changed() {
        let store = InMemoryGraphStore::new();
        let mut sub = store.subscribe(SubscribeOpts::default());

        let s = slot("com.acme.n1", "out");
        store
            .write_slot(&s, SlotValue::Int(7), WriteSlotOpts::replay())
            .await
            .unwrap();

        let none = timeout(Duration::from_millis(100), sub.next()).await;
        assert!(none.is_err(), "replay write must not emit: {none:?}");

        // …but the value is still readable through `read_slot`.
        let v = store.read_slot(&s).await.unwrap();
        assert_eq!(v, SlotValue::Int(7));
    }

    /// Writing the same value twice emits only one event — the R3
    /// idempotent-write short-circuit.
    #[tokio::test]
    async fn idempotent_write_short_circuits() {
        let store = InMemoryGraphStore::new();
        let mut sub = store.subscribe(SubscribeOpts::default());

        let s = slot("com.acme.n1", "out");
        store
            .write_slot(&s, SlotValue::Int(1), WriteSlotOpts::live())
            .await
            .unwrap();
        store
            .write_slot(&s, SlotValue::Int(1), WriteSlotOpts::live())
            .await
            .unwrap();

        // Exactly one event.
        let first = timeout(Duration::from_millis(200), sub.next())
            .await
            .expect("first write missing")
            .expect("stream ended");
        assert_eq!(first.value, Some(SlotValue::Int(1)));

        let none = timeout(Duration::from_millis(50), sub.next()).await;
        assert!(none.is_err(), "idempotent re-write must not emit: {none:?}");
    }

    /// `force = true` defeats the R3 short-circuit and emits anyway.
    #[tokio::test]
    async fn force_defeats_idempotent_short_circuit() {
        let store = InMemoryGraphStore::new();
        let mut sub = store.subscribe(SubscribeOpts::default());

        let s = slot("com.acme.n1", "out");
        store
            .write_slot(&s, SlotValue::Int(1), WriteSlotOpts::live())
            .await
            .unwrap();
        store
            .write_slot(&s, SlotValue::Int(1), WriteSlotOpts::forced())
            .await
            .unwrap();

        // Two events.
        let first = timeout(Duration::from_millis(200), sub.next())
            .await
            .expect("first write missing")
            .expect("stream ended");
        assert_eq!(first.value, Some(SlotValue::Int(1)));
        let second = timeout(Duration::from_millis(200), sub.next())
            .await
            .expect("forced write missing")
            .expect("stream ended");
        assert_eq!(second.value, Some(SlotValue::Int(1)));
    }

    /// A subscriber sees writes that happen after `subscribe`, never
    /// writes that happened before.
    #[tokio::test]
    async fn subscribe_sees_writes_after_only() {
        let store = InMemoryGraphStore::new();

        let s = slot("com.acme.n1", "out");
        // Pre-subscription write.
        store
            .write_slot(&s, SlotValue::Int(100), WriteSlotOpts::live())
            .await
            .unwrap();

        let mut sub = store.subscribe(SubscribeOpts::default());

        // No backfill — subscriber must not see the pre-write.
        let none = timeout(Duration::from_millis(50), sub.next()).await;
        assert!(
            none.is_err(),
            "subscriber must not see pre-subscription writes: {none:?}"
        );

        // Post-subscription write — different value so the R3
        // short-circuit doesn't swallow it.
        store
            .write_slot(&s, SlotValue::Int(101), WriteSlotOpts::live())
            .await
            .unwrap();

        let ev = timeout(Duration::from_millis(200), sub.next())
            .await
            .expect("post-subscribe write missing")
            .expect("stream ended");
        assert_eq!(ev.value, Some(SlotValue::Int(101)));
    }
}
