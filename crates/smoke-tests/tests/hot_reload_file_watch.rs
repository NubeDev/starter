//! HR7 smoke test: **File-watch is just another publisher**
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests" section).
//!
//! A flow file on disk is created externally; the watch adapter
//! parses it, calls `DefinitionManager::publish` with a `source =
//! File { path }`. The audit row records the `File` source. The
//! publish behaves identically to a REST publish (same
//! `flow.definition.publish` span shape, same `RevisionPublished`
//! event).
//!
//! Wired through the real public surface — `Engine`,
//! `DefinitionManager`, `SqliteFlowStore` (real migrations,
//! in-memory pool), and the `starter-flow-watch` adapter calling
//! `apply_file_event` directly (no `notify` dependency in the
//! test — that lives behind the `watch` cargo feature and is
//! exercised by `starter-flow-watch`'s own integration tests).

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tempfile::TempDir;
use tokio::time::timeout;

use starter_flow::definition::DefinitionManager;
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::definition::{DefinitionSource, FlowDefinitionEvent};
use starter_flow_spi::flow::{FlowId, FlowStore};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
use starter_flow_watch::{apply_file_event, FileEvent};
use starter_store_sqlite::flow::{SqliteFlowStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral};

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

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        Ok(input)
    }
}

async fn build_engine() -> (Engine, Arc<DefinitionManager>, Arc<dyn FlowStore>) {
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
    (engine, mgr, store)
}

/// HR7: dropping a flow file in a watched directory drives the
/// exact same publish chokepoint as a REST publish, with
/// `DefinitionSource::File { path }` recorded on the audit event.
#[tokio::test]
async fn hot_reload_file_watch_is_just_another_publisher() {
    let (engine, mgr, store) = build_engine().await;
    engine
        .register_kind(AnyKind::arc("com.acme.smoke.any"))
        .await
        .expect("register kind");

    // Write a flow file to a tempdir.
    let tmp = TempDir::new().expect("tempdir");
    let path = tmp.path().join("examples.smoke.fromfile.json");
    let body = serde_json::json!({
        "flow_id": "examples.smoke.fromfile",
        "nodes": [{"id": "smoke.n", "kind": "com.acme.smoke.any"}],
        "links": []
    });
    std::fs::write(&path, serde_json::to_vec_pretty(&body).unwrap()).expect("write flow file");

    let mut rx = mgr.subscribe();

    // Drive one Upsert through the same path the watcher uses.
    apply_file_event(&mgr, FileEvent::Upsert(path.clone()), |_| None)
        .await
        .expect("apply Upsert");

    // The bus must see a RevisionPublished with source=File{path}
    // — same shape as an API publish, just a different source tag.
    let mut saw_published_from_file = false;
    for _ in 0..6 {
        match timeout(Duration::from_millis(200), rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::RevisionPublished {
                source: DefinitionSource::File { path: p },
                ..
            })) => {
                assert_eq!(p, path, "audit source path must match the file path");
                saw_published_from_file = true;
                break;
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(
        saw_published_from_file,
        "watcher must publish through the same chokepoint with File source"
    );

    let flow = FlowId::new("examples.smoke.fromfile").unwrap();
    assert_eq!(
        store.revisions(flow.clone()).await.unwrap().len(),
        1,
        "file-watch publish must write exactly one revision"
    );
    assert_eq!(
        mgr.active_topologies().len().await,
        1,
        "file-watch publish must install the active topology"
    );

    // Deleting the file drives publish_delete with the same source
    // shape — the flow drops out of ActiveTopologies.
    let flow_for_remove = flow.clone();
    apply_file_event(&mgr, FileEvent::Remove(path.clone()), move |_p| {
        Some(flow_for_remove.clone())
    })
    .await
    .expect("apply Remove");
    assert_eq!(
        mgr.active_topologies().len().await,
        0,
        "Remove must drive publish_delete and revoke the mount"
    );
}
