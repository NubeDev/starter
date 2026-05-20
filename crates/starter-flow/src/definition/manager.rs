//! `DefinitionManager` — the HR1 publish chokepoint.
//!
//! Per `DOCS/flow/scope/hot-reload.md` HR1: every definition edit —
//! REST handler, CLI command, UI canvas save, host-dir file-watch,
//! extension reload, programmatic API — funnels through
//! [`DefinitionManager::publish`]. This module owns that one function
//! and the support types it produces.
//!
//! Phase HR-1 ships the chokepoint with:
//!
//! 1. Body parsing into the typed [`FlowBody`] shape.
//! 2. Per-node `validate_settings` via the kind's schema.
//! 3. Full [`TopologyResolver::resolve_body`] dry-run so the publish
//!    refuses anything that wouldn't mount.
//! 4. JCS canonicalisation + `blake3` hash for the idempotent
//!    short-circuit (HR1 step 3).
//! 5. Atomic write through [`FlowStore::put`] (HR1 step 4).
//! 6. [`FlowDefinitionEvent`] emission on the engine's definition
//!    bus.
//!
//! `ActiveTopology` swap (HR2), the diff classifier (HR2), per-flow
//! `apply_policy` dispatch (HR4), and the full observability surface
//! (HR3) layer on top in later phases without changing this
//! chokepoint's interface.

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use starter_flow_spi::definition::{
    ApplyPolicy, DefinitionSource, EditKindTag, FlowDefinitionEvent,
};
use starter_flow_spi::flow::{FlowError, FlowId, FlowRevision, FlowRevisionId, FlowStore};
use starter_flow_spi::graph::{GraphError, GraphStore, WriteSlotOpts};

use crate::definition::active::ActiveTopologies;
use crate::definition::body::{self, FlowBody};
use crate::definition::canonical::{body_hash, BodyHash};
use crate::definition::classifier::{classify, EditKind};
use crate::definition::resolver::{TopologyResolver, TopologyResolverError};
use crate::registry::NodeKindRegistry;

/// Default broadcast capacity for the definition bus.
///
/// Mirrors the `RunOpts::event_broadcast_capacity` shape from
/// `starter-flow-spi::flow` — sized so a slow consumer (a UI canvas
/// over a flaky network) can drop messages without back-pressuring
/// the publish call. The propagator's own broadcast also defaults
/// to 1024.
pub const DEFAULT_DEFINITION_BUS_CAPACITY: usize = 1024;

/// Outcome of a successful [`DefinitionManager::publish`] call.
///
/// Two terminal happy paths:
///
/// - [`Self::Published`] — the draft was a new revision; a fresh
///   [`FlowRevisionId`] is now durable in [`FlowStore`].
/// - [`Self::ShortCircuited`] — HR1's idempotent short-circuit hit;
///   the head's hash matched the draft's hash; no new revision was
///   written.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PublishOutcome {
    /// A new revision was written.
    Published {
        /// The newly-written revision id.
        revision: FlowRevisionId,
        /// The previous head, if any.
        prev_head: Option<FlowRevisionId>,
        /// Classifier output. Phase HR-1 always emits
        /// [`EditKindTag::Initial`] (first publish) or
        /// [`EditKindTag::Structural`] (every other publish) — the
        /// pure diff classifier lands HR-2.
        kind: EditKindTag,
    },
    /// The draft was identical to the current head; no new revision.
    ShortCircuited {
        /// The current head the draft collapsed onto.
        head: FlowRevisionId,
    },
}

/// Errors returned by [`DefinitionManager::publish`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PublishError {
    /// The draft body was syntactically invalid, referenced an
    /// unknown kind, or otherwise failed
    /// [`TopologyResolver::resolve_body`].
    #[error("resolve failed: {0}")]
    Resolve(#[from] TopologyResolverError),

    /// [`FlowStore`] rejected the write (backend unavailable,
    /// degraded, etc.). The previous head is unchanged.
    #[error("flow-store write failed: {0}")]
    Store(#[from] FlowError),

    /// A settings-path slot write failed against the live
    /// [`GraphStore`]. The new revision is durable in the
    /// [`FlowStore`] (the write that triggered this error happens
    /// AFTER the revision lands), and the active topology has been
    /// swapped if the edit was [`EditKind::Mixed`]; HR4's apply
    /// policy decides how the engine recovers.
    #[error("graph-store write failed: {0}")]
    Graph(#[from] GraphError),
}

/// The HR1 publish chokepoint.
///
/// Owns the dependencies the chokepoint needs (the persistence seam
/// + the kind registry) and the broadcast bus consumers subscribe to
/// for [`FlowDefinitionEvent`] notifications. Constructed by the
/// host binary at engine wire-up time and stored on the `Engine`
/// (the wire-up is Phase HR-2 work; HR-1 ships the manager as a
/// standalone unit that's easy to test).
pub struct DefinitionManager {
    store: Arc<dyn FlowStore>,
    kinds: Arc<NodeKindRegistry>,
    /// Optional live graph store. When set, settings-only edits and
    /// the settings half of mixed edits project onto live slots via
    /// [`GraphStore::write_slot`] with [`WriteSlotOpts::config`].
    /// When unset, the manager records the new revision and swaps
    /// the active topology but cannot apply settings deltas —
    /// useful in tools and tests that don't run a propagator.
    graph: Option<Arc<dyn GraphStore>>,
    /// Per-flow [`ActiveTopology`] registry. Structural and mixed
    /// edits install the freshly-resolved topology here; the engine
    /// run path resolves through [`ActiveTopologies::get`] so
    /// in-flight runs see new wiring on the next propagation step.
    active: Arc<ActiveTopologies>,
    events: broadcast::Sender<FlowDefinitionEvent>,
}

impl DefinitionManager {
    /// Construct a manager with the default broadcast capacity and
    /// no attached graph store. Settings-only edits will land in
    /// the [`FlowStore`] and update the active topology but will
    /// NOT project onto live slots until [`Self::attach_graph`]
    /// (or the [`Self::with_graph`] constructor) gives the manager
    /// a [`GraphStore`] handle.
    pub fn new(store: Arc<dyn FlowStore>, kinds: Arc<NodeKindRegistry>) -> Self {
        Self::with_capacity(store, kinds, DEFAULT_DEFINITION_BUS_CAPACITY)
    }

    /// Construct a manager with a custom broadcast capacity.
    pub fn with_capacity(
        store: Arc<dyn FlowStore>,
        kinds: Arc<NodeKindRegistry>,
        capacity: usize,
    ) -> Self {
        let (events, _) = broadcast::channel(capacity.max(1));
        Self {
            store,
            kinds,
            graph: None,
            active: Arc::new(ActiveTopologies::new()),
            events,
        }
    }

    /// Convenience constructor: build a manager pre-wired to a
    /// [`GraphStore`] so the settings path can execute.
    pub fn with_graph(
        store: Arc<dyn FlowStore>,
        kinds: Arc<NodeKindRegistry>,
        graph: Arc<dyn GraphStore>,
    ) -> Self {
        let mut mgr = Self::new(store, kinds);
        mgr.graph = Some(graph);
        mgr
    }

    /// Attach (or replace) the live [`GraphStore`] used by the
    /// settings path. Returns the previous graph store, if any.
    pub fn attach_graph(&mut self, graph: Arc<dyn GraphStore>) -> Option<Arc<dyn GraphStore>> {
        self.graph.replace(graph)
    }

    /// Borrow the [`ActiveTopologies`] registry the manager swaps
    /// through. Engines hand this to the run path so per-step slot
    /// reads see the freshest topology.
    pub fn active_topologies(&self) -> Arc<ActiveTopologies> {
        self.active.clone()
    }

    /// Borrow the [`FlowStore`] this manager writes through.
    pub fn store(&self) -> &Arc<dyn FlowStore> {
        &self.store
    }

    /// Borrow the [`NodeKindRegistry`] this manager resolves
    /// against.
    pub fn kinds(&self) -> &Arc<NodeKindRegistry> {
        &self.kinds
    }

    /// Subscribe to [`FlowDefinitionEvent`]s emitted by this
    /// manager. Each subscriber gets its own receiver; messages are
    /// dropped on slow subscribers per the `broadcast` channel
    /// contract.
    pub fn subscribe(&self) -> broadcast::Receiver<FlowDefinitionEvent> {
        self.events.subscribe()
    }

    /// Borrow the underlying broadcast sender. Useful for tests that
    /// want to assert subscriber count; production code calls
    /// [`Self::subscribe`].
    pub fn event_sender(&self) -> &broadcast::Sender<FlowDefinitionEvent> {
        &self.events
    }

    /// Publish a draft body for `flow_id`. Returns the [`FlowRevisionId`]
    /// that is now (or already was) the flow's head, along with the
    /// classifier tag for tracing / event emission.
    ///
    /// Full contract (HR1 + HR2):
    ///
    /// 1. Parse the body into the typed [`FlowBody`] shape.
    /// 2. Resolve — every node's kind must be registered, every
    ///    node's settings must pass `validate_settings`, every link
    ///    endpoint must reference a declared node. Produces an
    ///    `Arc<FlowTopology>` ready to mount.
    /// 3. Canonicalise (RFC 8785 JCS) + blake3 hash.
    /// 4. Look up the current head; if its body hashes to the same
    ///    value, short-circuit (no `FlowStore` write, no swap, just
    ///    [`FlowDefinitionEvent::PublishShortCircuited`]).
    /// 5. Allocate a fresh [`FlowRevisionId`] and write through
    ///    [`FlowStore::put`].
    /// 6. Classify the edit relative to the previous head
    ///    (Initial / SettingsOnly / Structural / Mixed / Unchanged).
    /// 7. Apply per the classifier:
    ///    - Initial / Structural / Mixed — install the
    ///      freshly-resolved topology into [`ActiveTopologies`]
    ///      (atomic `ArcSwap` swap if previously mounted) and emit
    ///      [`FlowDefinitionEvent::SwapApplied`] carrying the
    ///      [`ApplyPolicy`] read from the *previous* body (HR4:
    ///      the body being torn down dictates how).
    ///    - SettingsOnly / Mixed — project the per-field deltas
    ///      onto the attached [`GraphStore`] via
    ///      [`WriteSlotOpts::config`]. HR3 order: structural swap
    ///      first, then writes.
    /// 8. Emit [`FlowDefinitionEvent::RevisionPublished`] tagged
    ///    with the classifier output.
    pub async fn publish(
        &self,
        flow_id: FlowId,
        body: serde_json::Value,
        source: DefinitionSource,
    ) -> Result<PublishOutcome, PublishError> {
        // Step 1: parse the typed body.
        let parsed: FlowBody = match body::parse_body(&body) {
            Ok(b) => b,
            Err(e) => {
                let err = TopologyResolverError::BodyShape {
                    detail: e.to_string(),
                };
                self.emit_rejected(&flow_id, &source, &err);
                return Err(err.into());
            }
        };

        // Step 2: resolve — keep the topology for the HR2 mount.
        let topology = match TopologyResolver::resolve_body(&parsed, &flow_id, &self.kinds).await {
            Ok(t) => t,
            Err(e) => {
                self.emit_rejected(&flow_id, &source, &e);
                return Err(e.into());
            }
        };

        // Step 3: canonicalise + hash.
        let draft_hash = body_hash(&body);

        // Step 4: look up the head + load it once for both the
        // short-circuit and the diff classifier.
        let prev_head = self.store.head(flow_id.clone()).await?;
        let prev_revision = match prev_head.as_ref() {
            Some(head) => Some(self.store.load(flow_id.clone(), Some(*head)).await?),
            None => None,
        };

        if let Some(prev) = prev_revision.as_ref() {
            let head_hash = body_hash(&prev.body);
            if head_hash == draft_hash {
                debug!(
                    target: "starter_flow::definition",
                    flow = %flow_id,
                    head = %prev.revision_id,
                    body_hash = %draft_hash,
                    source = %source.audit_tag(),
                    "publish short-circuited: draft body hash matches head"
                );
                let _ = self.events.send(FlowDefinitionEvent::PublishShortCircuited {
                    flow: flow_id.clone(),
                    head: prev.revision_id,
                    source,
                });
                return Ok(PublishOutcome::ShortCircuited {
                    head: prev.revision_id,
                });
            }
        }

        // Step 6 (preview): classify against the previous body.
        let edit = match prev_revision.as_ref() {
            None => EditKind::Initial,
            Some(prev) => match body::parse_body(&prev.body) {
                Ok(prev_parsed) => classify(&prev_parsed, &parsed),
                Err(e) => {
                    warn!(
                        target: "starter_flow::definition",
                        flow = %flow_id,
                        head = %prev.revision_id,
                        error = %e,
                        "prev head body failed to re-parse; treating as Structural"
                    );
                    EditKind::Structural
                }
            },
        };
        let kind_tag = edit.tag();

        // Step 5: write a fresh revision BEFORE any side-effects.
        let revision_id = FlowRevisionId::new();
        let revision = FlowRevision::new(flow_id.clone(), revision_id, body);
        let written = self.store.put(revision).await?;

        // Emit RevisionPublished FIRST so consumers observe the
        // logical order revision-committed → topology-mounted →
        // settings-projected. Bus-driven UIs rely on this ordering
        // to update their canvas before re-painting per-slot state.
        info!(
            target: "starter_flow::definition",
            flow = %flow_id,
            revision = %written,
            prev_head = ?prev_head.as_ref().map(ToString::to_string),
            source = %source.audit_tag(),
            kind = ?kind_tag,
            body_hash = %draft_hash,
            "publish accepted: new flow revision written"
        );
        let _ = self.events.send(FlowDefinitionEvent::RevisionPublished {
            flow: flow_id.clone(),
            revision: written,
            prev_head,
            source: source.clone(),
            kind: kind_tag,
        });

        // Step 7: dispatch on the classifier.
        let apply_policy = prev_revision
            .as_ref()
            .and_then(|prev| body::parse_body(&prev.body).ok())
            .map(|prev_parsed| prev_parsed.apply_policy)
            .unwrap_or_default();

        let do_swap = matches!(
            edit,
            EditKind::Initial | EditKind::Structural | EditKind::Mixed { .. }
        );
        let settings_writes: Vec<_> = match &edit {
            EditKind::SettingsOnly { writes } | EditKind::Mixed { writes } => writes.clone(),
            _ => Vec::new(),
        };

        if do_swap {
            self.swap_topology(
                flow_id.clone(),
                topology,
                written,
                prev_head,
                apply_policy,
                &source,
                &edit,
            )
            .await;
        }

        // HR3 order: swap first, then settings writes — so writes
        // land in the new topology's slot graph. (The active
        // topology is the same process-wide `InMemoryGraphStore`
        // today, but the ordering contract is set now so the
        // per-flow graph store HR7 may introduce can honour it
        // without re-shuffling the publish path.)
        if !settings_writes.is_empty() {
            self.apply_settings(&flow_id, &settings_writes).await?;
        }

        debug!(
            target: "starter_flow::definition",
            flow = %flow_id,
            revision = %written,
            settings_writes = settings_writes.len(),
            "publish dispatch complete"
        );

        Ok(PublishOutcome::Published {
            revision: written,
            prev_head,
            kind: kind_tag,
        })
    }

    /// Install a freshly-resolved topology and emit the swap event.
    /// Initial mounts also emit a `Mounted` event so consumers can
    /// distinguish first-time mount from subsequent swaps.
    async fn swap_topology(
        &self,
        flow_id: FlowId,
        topology: Arc<crate::propagator::FlowTopology>,
        new_revision: FlowRevisionId,
        prev_head: Option<FlowRevisionId>,
        apply_policy: ApplyPolicy,
        source: &DefinitionSource,
        edit: &EditKind,
    ) {
        self.active.install(flow_id.clone(), topology).await;

        match prev_head {
            None => {
                info!(
                    target: "starter_flow::definition",
                    flow = %flow_id,
                    revision = %new_revision,
                    source = %source.audit_tag(),
                    "flow mounted (initial publish)"
                );
                let _ = self.events.send(FlowDefinitionEvent::Mounted {
                    flow: flow_id.clone(),
                    revision: new_revision,
                });
            }
            Some(prev) => {
                info!(
                    target: "starter_flow::definition",
                    flow = %flow_id,
                    from_revision = %prev,
                    to_revision = %new_revision,
                    apply_policy = ?apply_policy,
                    edit_kind = ?edit.tag(),
                    "active topology swapped"
                );
            }
        }

        let _ = self.events.send(FlowDefinitionEvent::SwapApplied {
            flow: flow_id,
            from_revision: prev_head,
            to_revision: new_revision,
            apply_policy,
        });
    }

    /// Project a settings delta onto the live [`GraphStore`].
    async fn apply_settings(
        &self,
        flow_id: &FlowId,
        writes: &[(starter_flow_spi::node::SlotRef, starter_flow_spi::node::SlotValue)],
    ) -> Result<(), PublishError> {
        let Some(graph) = self.graph.as_ref() else {
            debug!(
                target: "starter_flow::definition",
                flow = %flow_id,
                writes = writes.len(),
                "skipping settings projection: no GraphStore attached"
            );
            return Ok(());
        };
        for (slot, value) in writes {
            graph
                .write_slot(slot, value.clone(), WriteSlotOpts::config())
                .await
                .map_err(PublishError::Graph)?;
        }
        Ok(())
    }

    /// Compute the canonical-body blake3 hash for a value without
    /// publishing. Exposed for tests + future HR-2 callers that
    /// want to compare hashes outside the publish flow.
    pub fn hash_body(body: &serde_json::Value) -> BodyHash {
        body_hash(body)
    }

    fn emit_rejected(
        &self,
        flow_id: &FlowId,
        source: &DefinitionSource,
        err: &TopologyResolverError,
    ) {
        warn!(
            target: "starter_flow::definition",
            flow = %flow_id,
            source = %source.audit_tag(),
            error = %err,
            "publish rejected"
        );
        let _ = self.events.send(FlowDefinitionEvent::Rejected {
            flow: flow_id.clone(),
            source: source.clone(),
            reason: err.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use starter_flow_spi::flow::FlowResult;
    use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};

    /// Minimal in-memory `FlowStore` for tests. Mirrors the shape
    /// the SQLite impl will land in Phase HR-3; sufficient for HR-1
    /// smoke coverage.
    #[derive(Default)]
    struct MemStore {
        // (flow_id → revisions in insertion order; last = head).
        inner: Mutex<HashMap<FlowId, Vec<FlowRevision>>>,
    }

    impl MemStore {
        fn new() -> Arc<Self> {
            Arc::new(Self::default())
        }

        fn revision_count(&self, flow: &FlowId) -> usize {
            self.inner
                .lock()
                .unwrap()
                .get(flow)
                .map(Vec::len)
                .unwrap_or(0)
        }
    }

    #[async_trait]
    impl FlowStore for MemStore {
        async fn load(
            &self,
            flow_id: FlowId,
            revision: Option<FlowRevisionId>,
        ) -> FlowResult<FlowRevision> {
            let guard = self.inner.lock().unwrap();
            let revs = guard.get(&flow_id).ok_or_else(|| FlowError::NotFound {
                kind: "flow",
                id: flow_id.to_string(),
            })?;
            let target = match revision {
                Some(r) => revs.iter().find(|x| x.revision_id == r).cloned(),
                None => revs.last().cloned(),
            };
            target.ok_or_else(|| FlowError::NotFound {
                kind: "revision",
                id: flow_id.to_string(),
            })
        }

        async fn put(&self, revision: FlowRevision) -> FlowResult<FlowRevisionId> {
            let mut guard = self.inner.lock().unwrap();
            let revs = guard.entry(revision.flow_id.clone()).or_default();
            let id = revision.revision_id;
            revs.push(revision);
            Ok(id)
        }

        async fn list(&self) -> FlowResult<Vec<FlowId>> {
            Ok(self.inner.lock().unwrap().keys().cloned().collect())
        }

        async fn revisions(&self, flow_id: FlowId) -> FlowResult<Vec<FlowRevisionId>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&flow_id)
                .map(|v| v.iter().rev().map(|r| r.revision_id).collect())
                .unwrap_or_default())
        }

        async fn head(&self, flow_id: FlowId) -> FlowResult<Option<FlowRevisionId>> {
            Ok(self
                .inner
                .lock()
                .unwrap()
                .get(&flow_id)
                .and_then(|v| v.last().map(|r| r.revision_id)))
        }
    }

    struct AnyKind {
        kind: KindId,
    }
    impl AnyKind {
        fn arc(s: &str) -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new(s).unwrap(),
            })
        }
    }
    #[async_trait]
    impl NodeBehavior for AnyKind {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            Ok(SlotMap::new())
        }
    }

    fn flow_id() -> FlowId {
        FlowId::new("examples.test.demo").unwrap()
    }

    fn body_v1() -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        })
    }

    fn body_v1_reordered_keys() -> serde_json::Value {
        // Same body, different key order at every level.
        serde_json::json!({
            "links": [{"to": "test.n2.in", "from": "test.n1.out"}],
            "nodes": [
                {"kind": "com.example.any", "id": "test.n1"},
                {"kind": "com.example.any", "id": "test.n2"}
            ],
            "flow_id": "examples.test.demo"
        })
    }

    async fn build_manager() -> (Arc<DefinitionManager>, Arc<MemStore>) {
        let store = MemStore::new();
        let kinds = Arc::new(NodeKindRegistry::new());
        kinds.register(AnyKind::arc("com.example.any")).await.unwrap();
        let mgr = Arc::new(DefinitionManager::new(store.clone(), kinds));
        (mgr, store)
    }

    /// HR1: idempotent publish is a no-op.
    #[tokio::test]
    async fn hr1_idempotent_publish_is_noop() {
        let (mgr, store) = build_manager().await;

        let mut rx = mgr.subscribe();

        let first = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .expect("first publish");
        let first_id = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };
        assert_eq!(store.revision_count(&flow_id()), 1);

        // Same body, same source — must short-circuit.
        let second = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .expect("second publish");
        assert_eq!(
            second,
            PublishOutcome::ShortCircuited { head: first_id },
            "duplicate publish must short-circuit"
        );
        assert_eq!(
            store.revision_count(&flow_id()),
            1,
            "short-circuit must not write a second revision"
        );

        // Bus shape: first publish emits RevisionPublished +
        // Mounted + SwapApplied (in that order); second publish
        // emits PublishShortCircuited. Never RevisionPublished
        // twice.
        let ev1 = rx.recv().await.expect("event 1");
        assert!(matches!(ev1, FlowDefinitionEvent::RevisionPublished { .. }));
        let ev2 = rx.recv().await.expect("event 2");
        assert!(matches!(ev2, FlowDefinitionEvent::Mounted { .. }));
        let ev3 = rx.recv().await.expect("event 3");
        assert!(matches!(ev3, FlowDefinitionEvent::SwapApplied { .. }));
        let ev4 = rx.recv().await.expect("event 4");
        assert!(matches!(
            ev4,
            FlowDefinitionEvent::PublishShortCircuited { .. }
        ));
    }

    /// HR1: bad revision never goes live.
    #[tokio::test]
    async fn hr1_bad_revision_never_goes_live() {
        let (mgr, store) = build_manager().await;
        // First publish lands a clean head so we can verify it
        // doesn't move on the bad publish.
        let first = mgr
            .publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let first_id = match first {
            PublishOutcome::Published { revision, .. } => revision,
            other => panic!("expected Published, got {other:?}"),
        };

        // Publish a body referencing an unregistered kind.
        let bad = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [{"id": "test.n1", "kind": "com.missing"}],
            "links": []
        });
        let err = mgr
            .publish(flow_id(), bad, DefinitionSource::Api)
            .await
            .expect_err("bad publish must error");
        assert!(matches!(
            err,
            PublishError::Resolve(TopologyResolverError::UnknownKind { .. })
        ));

        // Head is unchanged; no second row in the store.
        assert_eq!(store.revision_count(&flow_id()), 1);
        assert_eq!(store.head(flow_id()).await.unwrap(), Some(first_id));
    }

    /// HR1: canonicalisation collapses semantically-equal bodies in
    /// different key orders onto the same revision.
    #[tokio::test]
    async fn hr1_canonical_publish_dedupes_key_order() {
        let (mgr, store) = build_manager().await;

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let second = mgr
            .publish(flow_id(), body_v1_reordered_keys(), DefinitionSource::Api)
            .await
            .expect("re-ordered keys publish");
        assert!(matches!(second, PublishOutcome::ShortCircuited { .. }));
        assert_eq!(store.revision_count(&flow_id()), 1);
    }

    /// HR1: a structural delta (new node) writes a fresh revision.
    #[tokio::test]
    async fn hr1_structural_change_writes_new_revision() {
        let (mgr, store) = build_manager().await;
        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();

        let body_v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n2.out", "to": "test.n3.in"}
            ]
        });
        let r = mgr
            .publish(flow_id(), body_v2, DefinitionSource::Api)
            .await
            .expect("structural publish");
        match r {
            PublishOutcome::Published { kind, prev_head, .. } => {
                assert_eq!(kind, EditKindTag::Structural);
                assert!(prev_head.is_some());
            }
            other => panic!("expected Published, got {other:?}"),
        }
        assert_eq!(store.revision_count(&flow_id()), 2);
    }

    /// HR1: flow_id in body must match the publish target.
    #[tokio::test]
    async fn hr1_flow_id_mismatch_rejected() {
        let (mgr, _store) = build_manager().await;
        let bad = serde_json::json!({
            "flow_id": "examples.test.other",
            "nodes": [], "links": []
        });
        let err = mgr
            .publish(flow_id(), bad, DefinitionSource::Api)
            .await
            .expect_err("flow id mismatch");
        assert!(matches!(
            err,
            PublishError::Resolve(TopologyResolverError::FlowIdMismatch { .. })
        ));
    }

    // ===================================================================
    // HR-2 smoke tests
    // ===================================================================

    use crate::graph::InMemoryGraphStore;
    use starter_flow_spi::graph::SubscribeOpts;
    use starter_flow_spi::node::SlotValue;
    use futures::StreamExt;
    use std::time::Duration;
    use tokio::time::timeout;

    async fn build_manager_with_graph() -> (
        Arc<DefinitionManager>,
        Arc<MemStore>,
        Arc<InMemoryGraphStore>,
    ) {
        let store = MemStore::new();
        let kinds = Arc::new(NodeKindRegistry::new());
        kinds
            .register(AnyKind::arc("com.example.any"))
            .await
            .unwrap();
        let graph: Arc<InMemoryGraphStore> = Arc::new(InMemoryGraphStore::new());
        let mgr = Arc::new(DefinitionManager::with_graph(
            store.clone(),
            kinds,
            graph.clone(),
        ));
        (mgr, store, graph)
    }

    fn body_with_setting(value: &str) -> serde_json::Value {
        serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any",
                 "settings": {"prompt": value}},
                {"id": "test.n2", "kind": "com.example.any"}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        })
    }

    /// HR2: a settings-only edit fires exactly one `SlotChanged`
    /// event per delta and does NOT swap the active topology.
    #[tokio::test]
    async fn hr2_settings_edit_is_one_slot_write() {
        let (mgr, store, graph) = build_manager_with_graph().await;

        // Mount.
        mgr.publish(flow_id(), body_with_setting("old"), DefinitionSource::Api)
            .await
            .unwrap();
        let active_after_mount = mgr
            .active_topologies()
            .get(&flow_id())
            .await
            .expect("flow mounted")
            .load();

        // Subscribe to the graph AFTER the mount so we don't see
        // any initial-projection writes (HR-2's resolver doesn't
        // do an initial projection yet — that's HR-7 follow-up —
        // but the subscription point keeps the test honest).
        let mut graph_rx = graph.subscribe(SubscribeOpts::default());

        // Subscribe to the definition bus.
        let mut def_rx = mgr.subscribe();

        // Settings-only edit.
        let out = mgr
            .publish(flow_id(), body_with_setting("new"), DefinitionSource::Api)
            .await
            .expect("settings publish");
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Settings,
                ..
            }
        ));
        // A new revision did land (the canonical hash differs).
        assert_eq!(store.revision_count(&flow_id()), 2);

        // The active topology pointer must NOT have been swapped.
        let active_after_edit = mgr
            .active_topologies()
            .get(&flow_id())
            .await
            .expect("still mounted")
            .load();
        assert!(
            Arc::ptr_eq(&active_after_mount, &active_after_edit),
            "settings-only edit must not swap the active topology"
        );

        // Exactly one SlotChanged event for the prompt slot.
        let ev = timeout(Duration::from_millis(200), graph_rx.next())
            .await
            .expect("graph event")
            .expect("envelope");
        let value = ev.value.expect("event carries a value");
        assert_eq!(ev.slot.node.as_str(), "test.n1");
        assert_eq!(ev.slot.slot, "prompt");
        assert!(matches!(value, SlotValue::String(s) if s == "new"));

        // RevisionPublished tagged Settings; NO SwapApplied (the
        // mount's own SwapApplied was already consumed before
        // def_rx subscribed).
        let def_ev = def_rx.recv().await.expect("def event");
        assert!(matches!(
            def_ev,
            FlowDefinitionEvent::RevisionPublished {
                kind: EditKindTag::Settings,
                ..
            }
        ));
        // No further definition events for this publish.
        assert!(timeout(Duration::from_millis(50), def_rx.recv()).await.is_err());
    }

    /// HR2: a structural edit swaps the active topology in place
    /// and emits SwapApplied (no settings writes).
    #[tokio::test]
    async fn hr2_structural_edit_swaps_active_topology() {
        let (mgr, _store, graph) = build_manager_with_graph().await;

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();
        let active = mgr.active_topologies().get(&flow_id()).await.unwrap();
        let before = active.load();

        let mut graph_rx = graph.subscribe(SubscribeOpts::default());
        let mut def_rx = mgr.subscribe();

        let v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any"},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n2.out", "to": "test.n3.in"}
            ]
        });
        let out = mgr
            .publish(flow_id(), v2, DefinitionSource::Api)
            .await
            .unwrap();
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Structural,
                ..
            }
        ));

        let after = active.load();
        assert!(
            !Arc::ptr_eq(&before, &after),
            "structural edit must swap the topology pointer"
        );

        // No slot writes.
        assert!(timeout(Duration::from_millis(50), graph_rx.next()).await.is_err());

        // RevisionPublished + SwapApplied (in that order).
        let mut saw_published = false;
        let mut saw_swap = false;
        for _ in 0..2 {
            let ev = def_rx.recv().await.expect("def event");
            match ev {
                FlowDefinitionEvent::RevisionPublished { kind, .. } => {
                    assert_eq!(kind, EditKindTag::Structural);
                    saw_published = true;
                }
                FlowDefinitionEvent::SwapApplied {
                    from_revision,
                    apply_policy,
                    ..
                } => {
                    assert!(from_revision.is_some(), "second mount must have a prev");
                    assert_eq!(apply_policy, ApplyPolicy::Drain);
                    saw_swap = true;
                }
                other => panic!("unexpected event: {other:?}"),
            }
        }
        assert!(saw_published && saw_swap);
    }

    /// HR2: a mixed edit (new node + settings change on a
    /// wiring-stable node) fires SwapApplied AND the settings
    /// write.
    #[tokio::test]
    async fn hr2_mixed_edit_swaps_then_writes_settings() {
        let (mgr, _store, graph) = build_manager_with_graph().await;

        // Body with two nodes; n2 has a settings field we'll
        // edit. (n2's wiring will stay stable across the edit; n1
        // gains a new outbound link.)
        let v1 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any",
                 "settings": {"level": "info"}}
            ],
            "links": [{"from": "test.n1.out", "to": "test.n2.in"}]
        });
        mgr.publish(flow_id(), v1, DefinitionSource::Api).await.unwrap();

        let mut graph_rx = graph.subscribe(SubscribeOpts::default());

        // v2: add n3 + a new link off n1 (n1 wiring shifts), AND
        // change n2.level (n2 wiring stable).
        let v2 = serde_json::json!({
            "flow_id": "examples.test.demo",
            "nodes": [
                {"id": "test.n1", "kind": "com.example.any"},
                {"id": "test.n2", "kind": "com.example.any",
                 "settings": {"level": "debug"}},
                {"id": "test.n3", "kind": "com.example.any"}
            ],
            "links": [
                {"from": "test.n1.out", "to": "test.n2.in"},
                {"from": "test.n1.out", "to": "test.n3.in"}
            ]
        });
        let out = mgr.publish(flow_id(), v2, DefinitionSource::Api).await.unwrap();
        assert!(matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Mixed,
                ..
            }
        ));

        // Settings projection fires (only for wiring-stable n2).
        let ev = timeout(Duration::from_millis(200), graph_rx.next())
            .await
            .expect("graph event")
            .expect("envelope");
        let value = ev.value.expect("event carries a value");
        assert_eq!(ev.slot.node.as_str(), "test.n2");
        assert_eq!(ev.slot.slot, "level");
        assert!(matches!(value, SlotValue::String(s) if s == "debug"));
    }

    /// HR2: initial publish emits Mounted + SwapApplied with no
    /// previous revision.
    #[tokio::test]
    async fn hr2_initial_publish_emits_mounted() {
        let (mgr, _store, _graph) = build_manager_with_graph().await;
        let mut def_rx = mgr.subscribe();

        mgr.publish(flow_id(), body_v1(), DefinitionSource::Api)
            .await
            .unwrap();

        let mut saw_mounted = false;
        let mut saw_swap = false;
        let mut saw_published = false;
        for _ in 0..3 {
            let ev = def_rx.recv().await.expect("event");
            match ev {
                FlowDefinitionEvent::Mounted { .. } => saw_mounted = true,
                FlowDefinitionEvent::SwapApplied { from_revision, .. } => {
                    assert!(from_revision.is_none());
                    saw_swap = true;
                }
                FlowDefinitionEvent::RevisionPublished { kind, .. } => {
                    assert_eq!(kind, EditKindTag::Initial);
                    saw_published = true;
                }
                other => panic!("unexpected: {other:?}"),
            }
        }
        assert!(saw_mounted && saw_swap && saw_published);
    }
}
