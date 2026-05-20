//! HR4 live-migrate smoke test
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests" — "Live-migrate
//! falls back to restart when wiring shifts").
//!
//! A flow has `apply_policy: live-migrate`. Run R1 is in flight,
//! modelled here as a `RunCancel` registered against the head
//! revision (the apply-policy dispatch logic doesn't care whether
//! a real propagator is driving the run — it only fires the
//! registered cancel handle).
//!
//! Two sub-cases:
//! - **A — settings-only**: operator changes a config slot on a
//!   node whose `(NodeId, KindId, inbound link set, outbound link
//!   set)` is unchanged. R1's `RunCancel` must NOT fire.
//! - **B — wiring shift**: operator changes wiring (adds a node +
//!   link). R1's `RunCancel` MUST fire (live-migrate falls back
//!   to restart for structural deltas).
//!
//! If sub-case B silently mutates the in-flight snapshot without
//! cancelling, HR4 live-migrate has slipped.

use std::sync::Arc;

use async_trait::async_trait;

use starter_flow::definition::{DefinitionManager, PublishOutcome};
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow::run::RunCancel;
use starter_flow_spi::Cancel;
use starter_flow_spi::definition::DefinitionSource;
use starter_flow_spi::flow::{FlowId, FlowStore};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap};
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

fn body_with(prompt: &str, structural: bool) -> serde_json::Value {
    let mut nodes = vec![
        serde_json::json!({"id": "smoke.agent", "kind": "com.acme.smoke.any",
                           "settings": {"prompt": prompt}}),
        serde_json::json!({"id": "smoke.log", "kind": "com.acme.smoke.any"}),
    ];
    let mut links = vec![
        serde_json::json!({"from": "smoke.agent.out", "to": "smoke.log.in"}),
    ];
    if structural {
        nodes.push(
            serde_json::json!({"id": "smoke.http_out", "kind": "com.acme.smoke.any"}),
        );
        links.push(
            serde_json::json!({"from": "smoke.agent.out", "to": "smoke.http_out.in"}),
        );
    }
    serde_json::json!({
        "flow_id": "examples.smoke.live_migrate",
        "apply_policy": "live-migrate",
        "nodes": nodes,
        "links": links,
    })
}

async fn build() -> (Engine, Arc<DefinitionManager>, Arc<dyn FlowStore>) {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let store: Arc<dyn FlowStore> = Arc::new(SqliteFlowStore::new(pool));
    let kinds = Arc::new(NodeKindRegistry::new());
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let mgr = Arc::new(DefinitionManager::with_graph(
        Arc::clone(&store),
        Arc::clone(&kinds),
        Arc::clone(&graph),
    ));
    let engine = Engine::new(graph)
        .with_node_kinds(Arc::clone(&kinds))
        .with_definition_manager(Arc::clone(&mgr));
    engine
        .register_kind(AnyKind::arc("com.acme.smoke.any"))
        .await
        .expect("register kind");
    (engine, mgr, store)
}

/// Sub-case A: settings-only edit under `live-migrate` must NOT
/// fire the in-flight run's cancel handle.
#[tokio::test]
async fn hot_reload_live_migrate_settings_only_does_not_cancel() {
    let (_engine, mgr, store) = build().await;
    let flow = FlowId::new("examples.smoke.live_migrate").unwrap();

    let first = mgr
        .publish(flow.clone(), body_with("old", false), DefinitionSource::Api)
        .await
        .expect("initial publish");
    let rev1 = match first {
        PublishOutcome::Published { revision, .. } => revision,
        other => panic!("expected Published, got {other:?}"),
    };

    // R1 in flight against rev1.
    let cancel = Arc::new(RunCancel::new());
    let _guard = mgr.register_run(flow.clone(), rev1, Arc::clone(&cancel));

    // Settings-only edit (wiring stable).
    let out = mgr
        .publish(flow.clone(), body_with("new", false), DefinitionSource::Api)
        .await
        .expect("settings publish");
    assert!(matches!(out, PublishOutcome::Published { .. }));

    assert!(
        !cancel.is_cancelled(),
        "live-migrate + settings-only must not fire RunCancel"
    );

    // Head moved forward — sanity.
    let head = store.head(flow.clone()).await.unwrap().expect("head");
    assert_ne!(head, rev1, "settings publish must still write a new revision");
}

/// Sub-case B: wiring shift under `live-migrate` MUST fall back to
/// restart and fire the in-flight run's cancel handle.
#[tokio::test]
async fn hot_reload_live_migrate_wiring_shift_cancels_in_flight() {
    let (_engine, mgr, _store) = build().await;
    let flow = FlowId::new("examples.smoke.live_migrate").unwrap();

    let first = mgr
        .publish(flow.clone(), body_with("old", false), DefinitionSource::Api)
        .await
        .expect("initial publish");
    let rev1 = match first {
        PublishOutcome::Published { revision, .. } => revision,
        other => panic!("expected Published, got {other:?}"),
    };

    let cancel = Arc::new(RunCancel::new());
    let _guard = mgr.register_run(flow.clone(), rev1, Arc::clone(&cancel));

    // Structural edit (adds a node + link) under live-migrate.
    let out = mgr
        .publish(flow.clone(), body_with("old", true), DefinitionSource::Api)
        .await
        .expect("structural publish");
    assert!(matches!(out, PublishOutcome::Published { .. }));

    assert!(
        cancel.is_cancelled(),
        "live-migrate + wiring shift must fall back to restart and fire RunCancel"
    );
}
