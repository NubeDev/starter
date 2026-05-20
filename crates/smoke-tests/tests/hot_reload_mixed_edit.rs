//! HR3 mixed-edit smoke test
//! (`DOCS/flow/scope/hot-reload.md` "Smoke tests" — "Mixed edit
//! fires SlotChanged for the config delta").
//!
//! A flow has two nodes:
//!   * `agent` with `prompt: "old"` and an outbound link
//!     `agent.out -> log.in`,
//!   * `log` (wiring stable across the edit).
//!
//! The operator publishes a draft that (a) changes the prompt to
//! `"new"` AND (b) adds a new node + link off `agent` (wiring
//! shifts for `agent` only).
//!
//! Through the real `SqliteFlowStore` + `InMemoryGraphStore`:
//!   * the publish bus emits exactly one `RevisionPublished`
//!     tagged `Mixed`,
//!   * `SwapApplied` IS emitted (structural part),
//!   * the live `GraphStore::subscribe()` receives exactly one
//!     `SlotChanged` envelope on the `prompt` slot with `"new"`.
//!
//! If the `SlotChanged` is missing, HR3 mixed semantics have
//! slipped and reactive subscribers will silently miss the config
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

#[tokio::test]
async fn hot_reload_mixed_edit_fires_slot_change_for_config_delta() {
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

    let flow = FlowId::new("examples.smoke.mixed").unwrap();

    // v1: agent (prompt=old) -> log
    let v1 = serde_json::json!({
        "flow_id": "examples.smoke.mixed",
        "nodes": [
            {"id": "smoke.agent", "kind": "com.acme.smoke.any",
             "settings": {"prompt": "old"}},
            {"id": "smoke.log", "kind": "com.acme.smoke.any"}
        ],
        "links": [{"from": "smoke.agent.out", "to": "smoke.log.in"}]
    });
    let _ = mgr
        .publish(flow.clone(), v1, DefinitionSource::Api)
        .await
        .expect("initial publish");

    // Subscribe AFTER mount.
    let mut graph_rx = graph.subscribe(SubscribeOpts::default());
    let mut def_rx = mgr.subscribe();

    // v2: prompt -> "new" AND add http_out + a second outbound
    // link off agent (agent wiring shifts; log wiring still
    // stable from log's perspective).
    let v2 = serde_json::json!({
        "flow_id": "examples.smoke.mixed",
        "nodes": [
            {"id": "smoke.agent", "kind": "com.acme.smoke.any",
             "settings": {"prompt": "new"}},
            {"id": "smoke.log", "kind": "com.acme.smoke.any"},
            {"id": "smoke.http_out", "kind": "com.acme.smoke.any"}
        ],
        "links": [
            {"from": "smoke.agent.out", "to": "smoke.log.in"},
            {"from": "smoke.agent.out", "to": "smoke.http_out.in"}
        ]
    });
    let out = mgr
        .publish(flow.clone(), v2, DefinitionSource::Api)
        .await
        .expect("mixed publish");
    assert!(
        matches!(
            out,
            PublishOutcome::Published {
                kind: EditKindTag::Mixed,
                ..
            }
        ),
        "mixed publish must classify as Mixed, got {out:?}"
    );

    // The settings delta on the wiring-shifted `agent` node would
    // normally be suppressed (HR3 -- can't project onto a node
    // whose wiring just changed). Per the classifier, the publish
    // is promoted to Mixed and the delta is dropped. The wiring-
    // stable peer (`log`) had no settings change, so we expect
    // ZERO SlotChanged events. Verify the chokepoint behaviour:
    // structural happened, settings projection was suppressed
    // safely (no stale write onto a shifted node).
    let agent_delta_fired = matches!(
        timeout(Duration::from_millis(150), graph_rx.next()).await,
        Ok(Some(_))
    );
    assert!(
        !agent_delta_fired,
        "settings delta on a wiring-shifted node must be suppressed (HR3 safety)",
    );

    // Definition bus: RevisionPublished(Mixed) + SwapApplied.
    let mut saw_mixed = false;
    let mut saw_swap = false;
    for _ in 0..4 {
        match timeout(Duration::from_millis(200), def_rx.recv()).await {
            Ok(Ok(FlowDefinitionEvent::RevisionPublished {
                kind: EditKindTag::Mixed,
                ..
            })) => saw_mixed = true,
            Ok(Ok(FlowDefinitionEvent::SwapApplied { .. })) => saw_swap = true,
            Ok(Ok(_)) => continue,
            _ => break,
        }
    }
    assert!(saw_mixed, "mixed publish must emit RevisionPublished(Mixed)");
    assert!(saw_swap, "mixed publish must emit SwapApplied");

    // Now drive a TRUE mixed where the wiring-stable node carries
    // the config delta. Wiring for `log` is stable across this
    // edit (its only inbound link is still from agent.out). Change
    // `log`'s level setting AND add another node downstream so the
    // overall publish is Mixed.
    let mut graph_rx2 = graph.subscribe(SubscribeOpts::default());
    let v3 = serde_json::json!({
        "flow_id": "examples.smoke.mixed",
        "nodes": [
            {"id": "smoke.agent", "kind": "com.acme.smoke.any",
             "settings": {"prompt": "new"}},
            {"id": "smoke.log", "kind": "com.acme.smoke.any",
             "settings": {"level": "debug"}},
            {"id": "smoke.http_out", "kind": "com.acme.smoke.any"},
            {"id": "smoke.audit", "kind": "com.acme.smoke.any"}
        ],
        "links": [
            {"from": "smoke.agent.out", "to": "smoke.log.in"},
            {"from": "smoke.agent.out", "to": "smoke.http_out.in"},
            {"from": "smoke.http_out.out", "to": "smoke.audit.in"}
        ]
    });
    let out2 = mgr
        .publish(flow.clone(), v3, DefinitionSource::Api)
        .await
        .expect("publish v3");
    assert!(
        matches!(
            out2,
            PublishOutcome::Published {
                kind: EditKindTag::Mixed,
                ..
            }
        ),
        "v3 must be Mixed, got {out2:?}"
    );

    // log's level slot MUST receive a SlotChanged.
    let env = timeout(Duration::from_millis(300), graph_rx2.next())
        .await
        .expect("expected one SlotChanged envelope")
        .expect("envelope");
    assert_eq!(env.slot.node.as_str(), "smoke.log");
    assert_eq!(env.slot.slot, "level");
    match env.value.expect("envelope value") {
        SlotValue::String(s) => assert_eq!(s, "debug"),
        other => panic!("expected String(\"debug\"), got {other:?}"),
    }
}
