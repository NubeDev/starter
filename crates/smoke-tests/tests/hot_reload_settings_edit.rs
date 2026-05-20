//! HR3 smoke test: **Settings edit is one slot write**
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests" section).
//!
//! A flow has a node with `prompt: "old"`. An operator publishes
//! a draft whose only delta is `prompt: "new"`. Through the real
//! `SqliteFlowStore` + `InMemoryGraphStore`:
//!
//! - the publish bus emits a single `RevisionPublished` tagged
//!   `Settings`,
//! - the live `GraphStore::subscribe()` receives exactly one
//!   `SlotChanged` envelope on the `prompt` slot,
//! - the active topology pointer is **not** swapped (HR3
//!   guarantees settings deltas project without re-resolving),
//! - no `SwapApplied` event lands on the bus.
//!
//! If a topology swap fires for a settings-only edit, HR3 has
//! slipped and reactive subscribers will miss the projected
//! delta.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::time::timeout;

use starter_flow::definition::{DefinitionManager, PublishOutcome};
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::definition::{DefinitionSource, EditKindTag, FlowDefinitionEvent};
use starter_flow_spi::flow::FlowId;
use starter_flow_spi::graph::{GraphStore, SubscribeOpts};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
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

fn body(prompt: &str) -> serde_json::Value {
    serde_json::json!({
        "flow_id": "examples.smoke.settings",
        "nodes": [
            {"id": "smoke.n", "kind": "com.acme.smoke.any",
             "settings": {"prompt": prompt}}
        ],
        "links": []
    })
}

#[tokio::test]
async fn hot_reload_settings_edit_is_one_slot_write() {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let flow_store = Arc::new(SqliteFlowStore::new(pool));
    let kinds = Arc::new(NodeKindRegistry::new());
    let graph: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let mgr = Arc::new(DefinitionManager::with_graph(
        flow_store,
        Arc::clone(&kinds),
        Arc::clone(&graph),
    ));
    let engine = Engine::new(Arc::clone(&graph))
        .with_node_kinds(Arc::clone(&kinds))
        .with_definition_manager(Arc::clone(&mgr));
    engine
        .register_kind(AnyKind::arc("com.acme.smoke.any"))
        .await
        .expect("register kind");

    // Mount with prompt="old".
    let flow = FlowId::new("examples.smoke.settings").unwrap();
    let _ = mgr
        .publish(flow.clone(), body("old"), DefinitionSource::Api)
        .await
        .expect("initial publish");
    let active_at_mount = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("mounted")
        .load();

    // Subscribe AFTER mount so we only observe the settings edit.
    let mut graph_rx = graph.subscribe(SubscribeOpts::default());
    let mut def_rx = mgr.subscribe();

    // Settings-only edit: prompt "old" -> "new".
    let out = mgr
        .publish(flow.clone(), body("new"), DefinitionSource::Api)
        .await
        .expect("settings publish");
    assert!(
        matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Settings,
                ..
            }
        ),
        "settings-only publish must classify as Settings, got {out:?}"
    );

    // Active topology pointer must NOT have swapped.
    let active_after_edit = mgr
        .active_topologies()
        .get(&flow)
        .await
        .expect("still mounted")
        .load();
    assert!(
        Arc::ptr_eq(&active_at_mount, &active_after_edit),
        "settings-only edit must not swap the active topology"
    );

    // Exactly one SlotChanged on the prompt slot.
    let env = timeout(Duration::from_millis(300), graph_rx.next())
        .await
        .expect("expected one SlotChanged envelope")
        .expect("envelope");
    assert_eq!(env.slot.node.as_str(), "smoke.n");
    assert_eq!(env.slot.slot, "prompt");
    match env.value.expect("envelope carries value") {
        SlotValue::String(s) => assert_eq!(s, "new"),
        other => panic!("expected String(\"new\"), got {other:?}"),
    }
    // No further graph events for this publish.
    assert!(
        timeout(Duration::from_millis(80), graph_rx.next())
            .await
            .is_err(),
        "settings edit must fire exactly one SlotChanged"
    );

    // Definition bus: RevisionPublished tagged Settings, NO SwapApplied.
    let def_ev = timeout(Duration::from_millis(200), def_rx.recv())
        .await
        .expect("def event")
        .expect("recv");
    assert!(
        matches!(
            def_ev,
            FlowDefinitionEvent::RevisionPublished {
                kind: EditKindTag::Settings,
                ..
            }
        ),
        "expected RevisionPublished(Settings), got {def_ev:?}"
    );
    // Drain anything else briefly and assert no SwapApplied.
    for _ in 0..3 {
        match timeout(Duration::from_millis(80), def_rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::SwapApplied { .. })) => {
                panic!("settings-only edit must NOT emit SwapApplied");
            }
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
}
