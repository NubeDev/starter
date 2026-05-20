//! Hot-reload smoke tests (HR section of
//! `DOCS/flow/scope/hot-reload.md`).
//!
//! Three end-to-end design checks from the "Smoke tests (before
//! merging)" section, wired through the real public surface:
//! [`Engine`] + [`DefinitionManager`] + [`SqliteFlowStore`] (real
//! migrations, in-memory pool):
//!
//! 1. **Idempotent publish is a no-op** (HR1) — publishing the same
//!    body twice returns the same [`FlowRevisionId`], adds no row to
//!    `flow_revisions`, emits no `SwapApplied`, emits
//!    `PublishShortCircuited`.
//! 2. **Bad revision never goes live** (HR6) — a draft referencing
//!    an unregistered [`KindId`] returns a typed error, leaves
//!    `FlowStore::head` untouched, and leaves the active topology
//!    (here, none) unchanged.
//! 3. **Kind deregister revokes affected flows** (HR8) — registering
//!    a kind mounts a flow that previously failed to resolve; the
//!    matching deregister revokes the active topology, emits
//!    [`FlowDefinitionEvent::KindRevoked`], and a subsequent
//!    re-register remounts the flow.
//!
//! These run against the real `SqliteFlowStore` (default-off `flow`
//! feature, already declared by the smoke-tests crate) so any future
//! drift in the SQLite migration schema or the publish chokepoint
//! breaks these tests loudly.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::time::timeout;

use starter_flow::definition::{DefinitionManager, PublishOutcome};
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::definition::{DefinitionSource, FlowDefinitionEvent};
use starter_flow_spi::flow::{FlowId, FlowStore};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
use starter_store_sqlite::flow::{SqliteFlowStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral};

/// A no-op node behavior parameterised on a [`KindId`]. The smoke
/// tests don't drive runs; they just need the resolver to find a
/// behavior matching the body's `kind` field.
struct AnyKind {
    kind: KindId,
}

impl AnyKind {
    fn arc(s: &str) -> Arc<Self> {
        Arc::new(Self {
            kind: KindId::new(s).expect("valid reverse-DNS kind id"),
        })
    }
}

#[async_trait]
impl NodeBehavior for AnyKind {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(
        &self,
        _ctx: NodeCtx<'_>,
        input: SlotMap,
    ) -> Result<SlotMap, NodeError> {
        Ok(input)
    }
}

async fn build_engine() -> (
    Engine,
    Arc<DefinitionManager>,
    Arc<dyn FlowStore>,
    Arc<NodeKindRegistry>,
) {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let store: Arc<dyn FlowStore> = Arc::new(SqliteFlowStore::new(pool));
    let kinds = Arc::new(NodeKindRegistry::new());
    let mgr = Arc::new(DefinitionManager::new(
        Arc::clone(&store),
        Arc::clone(&kinds),
    ));
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Engine::new(graph)
        .with_node_kinds(Arc::clone(&kinds))
        .with_definition_manager(Arc::clone(&mgr));
    (engine, mgr, store, kinds)
}

/// HR1: publishing the exact same body twice short-circuits — no
/// new revision, no swap, no `RevisionPublished` for the second
/// call. The second `publish` returns the same `FlowRevisionId`.
#[tokio::test]
async fn hot_reload_idempotent_publish_is_a_noop() {
    let (engine, mgr, store, _kinds) = build_engine().await;
    engine
        .register_kind(AnyKind::arc("com.acme.smoke.any"))
        .await
        .expect("register kind");

    let flow = FlowId::new("examples.smoke.idempotent").unwrap();
    let body = serde_json::json!({
        "flow_id": "examples.smoke.idempotent",
        "nodes": [{"id": "smoke.n", "kind": "com.acme.smoke.any"}],
        "links": []
    });

    let rev1 = mgr
        .publish(flow.clone(), body.clone(), DefinitionSource::Api)
        .await
        .expect("first publish");
    let rev1_id = match rev1 {
        PublishOutcome::Published { revision, .. } => revision,
        other => panic!("first publish must be Published, got {other:?}"),
    };

    let mut rx = mgr.subscribe();
    let rev2 = mgr
        .publish(flow.clone(), body.clone(), DefinitionSource::Api)
        .await
        .expect("second publish");
    let rev2_head = match rev2 {
        PublishOutcome::ShortCircuited { head } => head,
        other => panic!("second publish must be ShortCircuited, got {other:?}"),
    };

    assert_eq!(
        rev1_id, rev2_head,
        "idempotent publish must short-circuit onto the same revision"
    );
    assert_eq!(
        store.revisions(flow.clone()).await.unwrap().len(),
        1,
        "store must still have exactly one revision after idempotent publish",
    );

    // The bus must NOT see RevisionPublished or SwapApplied for the
    // second publish; it MUST see PublishShortCircuited.
    let mut saw_short_circuit = false;
    for _ in 0..6 {
        match timeout(Duration::from_millis(150), rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::PublishShortCircuited { .. })) => {
                saw_short_circuit = true;
            }
            Ok(Ok(FlowDefinitionEvent::RevisionPublished { .. })) => {
                panic!("idempotent publish must not emit RevisionPublished");
            }
            Ok(Ok(FlowDefinitionEvent::SwapApplied { .. })) => {
                panic!("idempotent publish must not emit SwapApplied");
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_short_circuit,
        "idempotent publish must emit PublishShortCircuited"
    );
}

/// HR6 second half: a draft referencing an unregistered kind is
/// rejected; `FlowStore::head` stays at the previous head and no
/// active topology is installed.
#[tokio::test]
async fn hot_reload_bad_revision_never_goes_live() {
    let (engine, mgr, store, _kinds) = build_engine().await;
    engine
        .register_kind(AnyKind::arc("com.acme.smoke.any"))
        .await
        .expect("register kind");

    let flow = FlowId::new("examples.smoke.bad").unwrap();
    let good = serde_json::json!({
        "flow_id": "examples.smoke.bad",
        "nodes": [{"id": "smoke.n", "kind": "com.acme.smoke.any"}],
        "links": []
    });
    let good_rev = match mgr
        .publish(flow.clone(), good, DefinitionSource::Api)
        .await
        .expect("good publish")
    {
        PublishOutcome::Published { revision, .. } => revision,
        other => panic!("good publish must be Published, got {other:?}"),
    };
    assert_eq!(engine.definitions().unwrap().active_topologies().len().await, 1);

    // Draft with an unknown kind.
    let bad = serde_json::json!({
        "flow_id": "examples.smoke.bad",
        "nodes": [{"id": "smoke.n", "kind": "com.acme.smoke.never_registered"}],
        "links": []
    });
    let err = mgr
        .publish(flow.clone(), bad, DefinitionSource::Api)
        .await
        .expect_err("bad publish must return typed error");
    let err_s = format!("{err:?}");
    assert!(
        err_s.contains("Resolve") || err_s.contains("UnknownKind") || err_s.contains("never_registered"),
        "expected resolve/unknown-kind error, got {err_s}"
    );

    // FlowStore::head must be unchanged.
    let head_after = store.head(flow.clone()).await.unwrap();
    assert_eq!(
        head_after,
        Some(good_rev),
        "FlowStore::head must still point at the good revision"
    );
    // ActiveTopology still mounted at the good revision.
    assert_eq!(engine.definitions().unwrap().active_topologies().len().await, 1);
}

/// HR8: deregister-then-re-register round-trips the flow's mount
/// state. After deregister the active topology is revoked and
/// `KindRevoked` is emitted; after re-register the manager re-walks
/// the failed set and remounts the flow, emitting `Mounted`.
#[tokio::test]
async fn hot_reload_kind_deregister_revokes_and_register_remounts() {
    let (engine, mgr, _store, _kinds) = build_engine().await;
    let kind_str = "com.acme.smoke.revoke";
    let kind = KindId::new(kind_str).unwrap();
    engine.register_kind(AnyKind::arc(kind_str)).await.unwrap();

    let flow = FlowId::new("examples.smoke.revoke").unwrap();
    mgr.publish(
        flow.clone(),
        serde_json::json!({
            "flow_id": "examples.smoke.revoke",
            "nodes": [{"id": "smoke.n", "kind": kind_str}],
            "links": []
        }),
        DefinitionSource::Api,
    )
    .await
    .expect("publish");
    assert_eq!(mgr.active_topologies().len().await, 1);

    // Deregister via the engine wiring -> revoke walk fires.
    let mut rx = mgr.subscribe();
    engine.deregister_kind(&kind).await.expect("deregister");
    assert_eq!(mgr.active_topologies().len().await, 0);
    assert_eq!(mgr.failed_flows().await.len(), 1);

    let mut saw_revoked = false;
    for _ in 0..6 {
        match timeout(Duration::from_millis(200), rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::KindRevoked { kind: k, .. })) => {
                assert_eq!(k, kind_str);
                saw_revoked = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_revoked, "KindRevoked must be emitted on deregister");

    // Re-register via the engine wiring -> remount walk fires.
    let mut rx2 = mgr.subscribe();
    engine.register_kind(AnyKind::arc(kind_str)).await.unwrap();

    let mut saw_mounted = false;
    for _ in 0..6 {
        match timeout(Duration::from_millis(200), rx2.recv()).await {
            Ok(Ok(FlowDefinitionEvent::Mounted { .. })) => {
                saw_mounted = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_mounted, "Mounted must be emitted on re-register");
    assert_eq!(mgr.active_topologies().len().await, 1);
    assert!(mgr.failed_flows().await.is_empty());
}
