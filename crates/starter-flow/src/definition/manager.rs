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
    DefinitionSource, EditKindTag, FlowDefinitionEvent,
};
use starter_flow_spi::flow::{FlowError, FlowId, FlowRevision, FlowRevisionId, FlowStore};

use crate::definition::body::{self, FlowBody};
use crate::definition::canonical::{body_hash, BodyHash};
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
    events: broadcast::Sender<FlowDefinitionEvent>,
}

impl DefinitionManager {
    /// Construct a manager with the default broadcast capacity.
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
            events,
        }
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
    /// HR1 contract (in order):
    ///
    /// 1. Validate — parse the body, refuse if the typed shape
    ///    doesn't match.
    /// 2. Resolve — every node's kind must be registered, every
    ///    node's settings must pass `validate_settings`, every link
    ///    endpoint must reference a declared node.
    /// 3. Canonicalise — RFC 8785 JCS over the body.
    /// 4. Hash — blake3 over the canonical bytes.
    /// 5. Look up the current head; if its body hashes to the same
    ///    value, short-circuit (no `FlowStore` write, no swap event,
    ///    a [`FlowDefinitionEvent::PublishShortCircuited`] notice).
    /// 6. Allocate a fresh [`FlowRevisionId`], write through
    ///    [`FlowStore::put`], emit
    ///    [`FlowDefinitionEvent::RevisionPublished`].
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

        // Step 2: resolve (kind lookup + settings validation + link
        // cross-check). We discard the resulting topology — Phase
        // HR-1 doesn't swap. Storing it on the manager is HR-2's
        // job.
        if let Err(e) =
            TopologyResolver::resolve_body(&parsed, &flow_id, &self.kinds).await
        {
            self.emit_rejected(&flow_id, &source, &e);
            return Err(e.into());
        }

        // Step 3 + 4: canonicalise + hash.
        let draft_hash = body_hash(&body);

        // Step 5: idempotent short-circuit against the head.
        let prev_head = self.store.head(flow_id.clone()).await?;
        if let Some(head) = prev_head.as_ref() {
            let head_revision = self.store.load(flow_id.clone(), Some(*head)).await?;
            let head_hash = body_hash(&head_revision.body);
            if head_hash == draft_hash {
                debug!(
                    target: "starter_flow::definition",
                    flow = %flow_id,
                    head = %head,
                    body_hash = %draft_hash,
                    source = %source.audit_tag(),
                    "publish short-circuited: draft body hash matches head"
                );
                let _ = self.events.send(FlowDefinitionEvent::PublishShortCircuited {
                    flow: flow_id.clone(),
                    head: *head,
                    source,
                });
                return Ok(PublishOutcome::ShortCircuited { head: *head });
            }
        }

        // Step 6: write a fresh revision.
        let revision_id = FlowRevisionId::new();
        let revision = FlowRevision::new(flow_id.clone(), revision_id, body);
        let written = self.store.put(revision).await?;
        let kind = if prev_head.is_some() {
            // Phase HR-1: every non-initial publish is reported as
            // `Structural`. The pure diff classifier lands HR-2 and
            // will refine this to `Settings | Structural | Mixed`.
            EditKindTag::Structural
        } else {
            EditKindTag::Initial
        };

        info!(
            target: "starter_flow::definition",
            flow = %flow_id,
            revision = %written,
            prev_head = ?prev_head.as_ref().map(ToString::to_string),
            source = %source.audit_tag(),
            kind = ?kind,
            body_hash = %draft_hash,
            "publish accepted: new flow revision written"
        );

        let _ = self.events.send(FlowDefinitionEvent::RevisionPublished {
            flow: flow_id,
            revision: written,
            prev_head,
            source,
            kind,
        });

        Ok(PublishOutcome::Published {
            revision: written,
            prev_head,
            kind,
        })
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

        // Bus shape: first publish emits RevisionPublished, second
        // emits PublishShortCircuited — never RevisionPublished
        // twice.
        let ev1 = rx.recv().await.expect("event 1");
        assert!(matches!(ev1, FlowDefinitionEvent::RevisionPublished { .. }));
        let ev2 = rx.recv().await.expect("event 2");
        assert!(matches!(
            ev2,
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
}
