//! `NodeKindRegistry` + `FlowRegistry`.
//!
//! SCOPE section: "Phase 2 — `starter-flow` engine" / "What lands in
//! `starter-flow`" — the two registries the engine resolves against at
//! run time:
//!
//! - [`NodeKindRegistry`] keyed by reverse-DNS [`KindId`], storing the
//!   [`NodeBehavior`] impl backing each kind. R10 namespace ownership
//!   is enforced at registration time: the reserved `starter.flow.*`
//!   prefix belongs to the host and only the host-owned
//!   [`NodeKindRegistry::register_builtin`] entry point may register
//!   under it. Every other caller must use
//!   [`NodeKindRegistry::register`], which refuses the reserved prefix
//!   regardless of who calls it.
//!
//! - [`FlowRegistry`] keyed by [`FlowId`] + [`FlowRevisionId`], storing
//!   the flow definitions the engine resolves into a runnable topology.
//!   Phase 2 holds revisions in a plain in-memory `Vec` per flow — the
//!   real persistence lands in Phase 3 alongside the SQLite `FlowStore`
//!   impl (see SCOPE phasing block). Multiple revisions per flow are
//!   supported today; revisions are immutable per SCOPE "Decisions
//!   made" so a duplicate revision-id registration is refused rather
//!   than silently overwriting.
//!
//! Both registries are guarded by `tokio::sync::RwLock` for the same
//! reason the [`crate::graph::InMemoryGraphStore`] is: the engine's
//! read paths are vastly more common than its write paths, and the
//! propagator must never block a reader waiting on a writer that is
//! waiting on the propagator.

use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::RwLock;

use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, NodeBehavior};

/// Reserved node-kind prefix owned by the host (SCOPE R10).
///
/// Only [`NodeKindRegistry::register_builtin`] may register a kind id
/// that starts with this prefix; [`NodeKindRegistry::register`]
/// refuses it unconditionally. The wider `starter.*` / `sys.*` /
/// `flow.*` reservation surface from R10 is not enforced here — Phase
/// 2 ships the engine-internal slice (built-in node kinds live under
/// `starter.flow.*`) and the extension-adapter boundary (R11) will
/// own the full set when `starter-ext-flow` lands in Phase 6.
pub const RESERVED_KIND_PREFIX: &str = "starter.flow.";

/// Errors raised by the two registries.
///
/// `#[non_exhaustive]` so new failure modes from Phase 3 (e.g. a flow
/// definition referencing an unregistered kind) can be added without
/// breaking callers that pattern-match.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum RegistryError {
    /// A non-host caller attempted to register a kind under the
    /// reserved `starter.flow.*` prefix (R10).
    #[error("kind id {0} is in the reserved {RESERVED_KIND_PREFIX}* namespace")]
    ReservedNamespace(KindId),

    /// A [`KindId`] was registered more than once.
    #[error("kind id {0} is already registered")]
    DuplicateKind(KindId),

    /// A [`KindId`] was deregistered but is not present.
    #[error("kind id {0} is not registered")]
    UnknownKind(KindId),

    /// The same [`FlowRevisionId`] was registered twice under a
    /// [`FlowId`]. Revisions are immutable per SCOPE "Decisions made";
    /// re-registration is refused rather than overwriting.
    #[error("flow {flow} revision {revision} is already registered")]
    DuplicateRevision {
        /// The flow the duplicate revision targets.
        flow: FlowId,
        /// The duplicate revision id.
        revision: FlowRevisionId,
    },
}

/// Registry of [`NodeBehavior`] impls keyed by [`KindId`].
///
/// See the module-level docs for the R10 namespace-ownership contract
/// this enforces (`register` vs `register_builtin`).
pub struct NodeKindRegistry {
    inner: RwLock<HashMap<KindId, Arc<dyn NodeBehavior>>>,
}

impl Default for NodeKindRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeKindRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Register a kind from outside the host (e.g. an extension via
    /// the future `starter-ext-flow` adapter).
    ///
    /// Refuses any [`KindId`] starting with [`RESERVED_KIND_PREFIX`]
    /// per R10, and refuses a duplicate id whose kind is already
    /// registered.
    pub async fn register(&self, behavior: Arc<dyn NodeBehavior>) -> Result<(), RegistryError> {
        let kind = behavior.kind_id().clone();
        if kind.as_str().starts_with(RESERVED_KIND_PREFIX) {
            return Err(RegistryError::ReservedNamespace(kind));
        }
        self.insert(kind, behavior).await
    }

    /// Host-only registration path. Accepts kinds under the reserved
    /// `starter.flow.*` prefix (which is the *only* difference between
    /// this and [`Self::register`]); duplicate detection still
    /// applies.
    ///
    /// Per SCOPE R10 the host is the sole caller. The crate's public
    /// surface places no further restriction — visibility is enforced
    /// by convention and code review, the way `starter-spi` already
    /// handles its host-only seams. The engine wires built-in kinds
    /// from `starter-flow-nodes` through this path; nothing else
    /// should.
    pub async fn register_builtin(
        &self,
        behavior: Arc<dyn NodeBehavior>,
    ) -> Result<(), RegistryError> {
        let kind = behavior.kind_id().clone();
        self.insert(kind, behavior).await
    }

    async fn insert(
        &self,
        kind: KindId,
        behavior: Arc<dyn NodeBehavior>,
    ) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;
        if map.contains_key(&kind) {
            return Err(RegistryError::DuplicateKind(kind));
        }
        map.insert(kind, behavior);
        Ok(())
    }

    /// Look up the [`NodeBehavior`] for a kind. Returns `None` if no
    /// kind is registered under the id.
    pub async fn lookup(&self, kind: &KindId) -> Option<Arc<dyn NodeBehavior>> {
        let map = self.inner.read().await;
        map.get(kind).cloned()
    }

    /// Deregister a kind. Returns [`RegistryError::UnknownKind`] if
    /// no kind is registered under the id.
    pub async fn deregister(&self, kind: &KindId) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;
        if map.remove(kind).is_some() {
            Ok(())
        } else {
            Err(RegistryError::UnknownKind(kind.clone()))
        }
    }

    /// Count the registered kinds (test / inspector convenience).
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }

    /// Whether the registry is empty.
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

/// A flow definition tracked by the [`FlowRegistry`].
///
/// Phase 2 ships the *shape* the registry needs (the [`FlowId`] +
/// [`FlowRevisionId`] pair and a placeholder body) — the in-engine
/// representation of nodes, links, triggers, and policies lands in
/// Phase 3 alongside `FlowStore`. Keeping a minimal struct here lets
/// stage 5 commit a working registry without pre-deciding the
/// persistence schema.
///
/// `#[non_exhaustive]` so Phase 3 can add fields (nodes, links,
/// auth, etc.) without breaking callers that construct or pattern-match.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FlowDefinition {
    /// The flow id.
    pub flow: FlowId,
    /// This revision's id. Revisions are immutable per SCOPE
    /// "Decisions made"; the registry refuses re-registration under
    /// the same `(flow, revision)` pair.
    pub revision: FlowRevisionId,
}

impl FlowDefinition {
    /// Construct a minimal [`FlowDefinition`]. Phase 3 will replace
    /// this with a richer builder once the nodes/links shape lands.
    pub fn new(flow: FlowId, revision: FlowRevisionId) -> Self {
        Self { flow, revision }
    }
}

/// Registry of flow definitions keyed by [`FlowId`] → revisions.
///
/// See the module-level docs for the Phase 2 / Phase 3 boundary: this
/// holds a thin in-memory `Vec` of revisions per flow today; the
/// SQLite-backed persistence ships in Phase 3.
pub struct FlowRegistry {
    inner: RwLock<HashMap<FlowId, Vec<FlowDefinition>>>,
}

impl Default for FlowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new revision of a flow.
    ///
    /// Multiple revisions per [`FlowId`] are supported; the most
    /// recently registered revision is treated as the head. A
    /// `(flow, revision)` pair that already exists returns
    /// [`RegistryError::DuplicateRevision`] — revisions are immutable
    /// per SCOPE "Decisions made".
    pub async fn put(&self, def: FlowDefinition) -> Result<(), RegistryError> {
        let mut map = self.inner.write().await;
        let revisions = map.entry(def.flow.clone()).or_default();
        if revisions.iter().any(|d| d.revision == def.revision) {
            return Err(RegistryError::DuplicateRevision {
                flow: def.flow,
                revision: def.revision,
            });
        }
        revisions.push(def);
        Ok(())
    }

    /// Look up a specific revision of a flow. Returns `None` if either
    /// the flow id or the revision id is unknown.
    pub async fn lookup(&self, flow: &FlowId, revision: &FlowRevisionId) -> Option<FlowDefinition> {
        let map = self.inner.read().await;
        map.get(flow)
            .and_then(|revs| revs.iter().find(|d| d.revision == *revision).cloned())
    }

    /// Return the head revision of a flow (the most recently
    /// registered one), or `None` if the flow has no revisions.
    pub async fn head(&self, flow: &FlowId) -> Option<FlowDefinition> {
        let map = self.inner.read().await;
        map.get(flow).and_then(|revs| revs.last().cloned())
    }

    /// List the revision ids for a flow in registration order.
    pub async fn revisions(&self, flow: &FlowId) -> Vec<FlowRevisionId> {
        let map = self.inner.read().await;
        map.get(flow)
            .map(|revs| revs.iter().map(|d| d.revision).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use starter_flow_spi::node::{NodeCtx, NodeError, SlotMap};

    /// Test behavior. Records its kind id; `invoke` is a no-op (the
    /// registry only cares about identity, not call shape).
    struct StubBehavior {
        kind: KindId,
    }

    impl StubBehavior {
        fn new(kind: &str) -> Arc<Self> {
            Arc::new(Self {
                kind: KindId::new(kind).unwrap(),
            })
        }
    }

    #[async_trait]
    impl NodeBehavior for StubBehavior {
        fn kind_id(&self) -> &KindId {
            &self.kind
        }
        async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
            Ok(SlotMap::new())
        }
    }

    /// Non-host `register` refuses a kind under the reserved
    /// `starter.flow.*` prefix (R10). The same id goes in cleanly
    /// through the host-only `register_builtin` path.
    #[tokio::test]
    async fn register_refuses_reserved_prefix_from_outside_host() {
        let registry = NodeKindRegistry::new();
        let behavior = StubBehavior::new("starter.flow.transform");

        let err = registry
            .register(behavior.clone())
            .await
            .expect_err("register must refuse reserved prefix");
        assert!(
            matches!(err, RegistryError::ReservedNamespace(ref k) if k.as_str() == "starter.flow.transform"),
            "expected ReservedNamespace; got {err:?}",
        );

        // The kind did NOT get registered.
        assert!(registry.lookup(behavior.kind_id()).await.is_none());
        assert!(registry.is_empty().await);

        // The host-only path accepts the same id.
        registry
            .register_builtin(behavior.clone())
            .await
            .expect("register_builtin must accept reserved prefix");
        assert!(registry.lookup(behavior.kind_id()).await.is_some());
    }

    /// Duplicate registration of the same [`KindId`] is refused, on
    /// both the public and the host-only paths.
    #[tokio::test]
    async fn duplicate_kind_registration_is_refused() {
        let registry = NodeKindRegistry::new();

        let one = StubBehavior::new("com.acme.weather");
        let two = StubBehavior::new("com.acme.weather");

        registry.register(one).await.unwrap();
        let err = registry
            .register(two)
            .await
            .expect_err("second register must fail");
        assert!(matches!(err, RegistryError::DuplicateKind(_)), "{err:?}");

        // Same story for the host-only path.
        let builtin_a = StubBehavior::new("starter.flow.transform");
        let builtin_b = StubBehavior::new("starter.flow.transform");
        registry.register_builtin(builtin_a).await.unwrap();
        let err = registry
            .register_builtin(builtin_b)
            .await
            .expect_err("second register_builtin must fail");
        assert!(matches!(err, RegistryError::DuplicateKind(_)), "{err:?}");
    }

    /// `lookup` after `register` returns the same `Arc`; `lookup`
    /// after `deregister` returns `None`.
    #[tokio::test]
    async fn lookup_after_register_then_deregister() {
        let registry = NodeKindRegistry::new();
        let behavior = StubBehavior::new("com.acme.weather");
        let registered: Arc<dyn NodeBehavior> = behavior.clone();

        registry.register(behavior.clone()).await.unwrap();
        let resolved = registry
            .lookup(behavior.kind_id())
            .await
            .expect("lookup-after-register must succeed");
        assert!(
            Arc::ptr_eq(&resolved, &registered),
            "lookup must return the same Arc identity that was registered",
        );

        registry.deregister(behavior.kind_id()).await.unwrap();
        assert!(
            registry.lookup(behavior.kind_id()).await.is_none(),
            "lookup-after-deregister must return None",
        );

        // Second deregister errors.
        let err = registry
            .deregister(behavior.kind_id())
            .await
            .expect_err("deregister twice must fail");
        assert!(matches!(err, RegistryError::UnknownKind(_)), "{err:?}");
    }

    /// `FlowRegistry` tracks multiple revisions per [`FlowId`] and
    /// returns the right body on lookup-by-revision.
    #[tokio::test]
    async fn flow_registry_holds_multiple_revisions_per_flow() {
        let registry = FlowRegistry::new();
        let flow = FlowId::new("com.acme.refund").unwrap();
        let rev_a = FlowRevisionId::new();
        let rev_b = FlowRevisionId::new();
        let rev_c = FlowRevisionId::new();

        registry
            .put(FlowDefinition::new(flow.clone(), rev_a))
            .await
            .unwrap();
        registry
            .put(FlowDefinition::new(flow.clone(), rev_b))
            .await
            .unwrap();
        registry
            .put(FlowDefinition::new(flow.clone(), rev_c))
            .await
            .unwrap();

        // All three revisions are recorded in registration order.
        let revs = registry.revisions(&flow).await;
        assert_eq!(revs, vec![rev_a, rev_b, rev_c]);

        // Head is the most recent.
        let head = registry.head(&flow).await.expect("head must exist");
        assert_eq!(head.revision, rev_c);

        // Lookup-by-revision returns the right body for each.
        for rev in [rev_a, rev_b, rev_c] {
            let def = registry
                .lookup(&flow, &rev)
                .await
                .expect("lookup must find revision");
            assert_eq!(def.flow, flow);
            assert_eq!(def.revision, rev);
        }

        // Unknown revision returns None.
        let unknown_rev = FlowRevisionId::new();
        assert!(registry.lookup(&flow, &unknown_rev).await.is_none());

        // Unknown flow returns None / empty.
        let other_flow = FlowId::new("com.acme.other").unwrap();
        assert!(registry.lookup(&other_flow, &rev_a).await.is_none());
        assert!(registry.head(&other_flow).await.is_none());
        assert!(registry.revisions(&other_flow).await.is_empty());

        // Duplicate revision under the same flow is refused.
        let err = registry
            .put(FlowDefinition::new(flow.clone(), rev_b))
            .await
            .expect_err("duplicate revision must fail");
        match err {
            RegistryError::DuplicateRevision {
                flow: ref f,
                revision: r,
            } => {
                assert_eq!(f, &flow);
                assert_eq!(r, rev_b);
            }
            other => panic!("expected DuplicateRevision; got {other:?}"),
        }
    }
}
