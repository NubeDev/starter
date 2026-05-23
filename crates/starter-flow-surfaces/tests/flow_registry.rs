//! Integration tests for `FlowRegistry::resolve` +
//! `FlowAsTool::from_registry` (U3 — see
//! `docs/design/starter-changes/README.md` Phase 2b).
//!
//! Mirrors the concerns of `crates/smoke-tests/tests/flow_via_mcp.rs`
//! while replacing the hand-rolled `FlowAsTool::builder()`
//! plumbing with one `from_registry` call — the point of U3 is
//! that wiring becomes one line.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tempfile::tempdir;

use starter_flow::definition::body::{FlowBody, LinkDecl, NodeDecl};
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_surfaces::{FlowAsTool, FlowRegistration, FlowRegistry, FlowRegistryError};
use starter_spi::tool::Tool;

/// A trivial node behavior — `out = in + 1` on the integer input
/// at slot `in`. Exists to give the registry a non-trivial body
/// to resolve and the test a deterministic expected output.
struct Increment {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for Increment {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // The default seed adapter writes the entire JSON input
        // value to the seed slot, so this node reads either a
        // bare integer or an object with a `value` field — the
        // latter matches the test's input_schema.
        let v = match input.get("in").cloned().unwrap_or(SlotValue::Null) {
            SlotValue::Json(Value::Number(n)) => n.as_i64().unwrap_or(0),
            SlotValue::Json(Value::Object(o)) => {
                o.get("value").and_then(|v| v.as_i64()).unwrap_or(0)
            }
            _ => 0,
        };
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), SlotValue::Json(serde_json::json!(v + 1)));
        Ok(out)
    }
}

fn increment_kind() -> Arc<Increment> {
    Arc::new(Increment {
        kind: KindId::new("test.flow-registry.increment").unwrap(),
    })
}

/// Build a `FlowBody` with one `Increment` node whose `in` slot
/// fires the node.
fn fixture_body(flow_id: &FlowId) -> FlowBody {
    let node = NodeId::new("test.inc").unwrap();
    let kind = KindId::new("test.flow-registry.increment").unwrap();
    let mut decl = NodeDecl::new(node, kind);
    decl.triggers = vec!["in".to_owned()];
    let mut body = FlowBody::new(flow_id.clone());
    body.nodes = vec![decl];
    body.links = Vec::<LinkDecl>::new();
    body
}

async fn fixture_kinds() -> NodeKindRegistry {
    let kinds = NodeKindRegistry::new();
    kinds.register(increment_kind()).await.unwrap();
    kinds
}

fn make_engine() -> Arc<Engine> {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    Arc::new(Engine::new(store))
}

fn registration(body: FlowBody, revision: FlowRevisionId) -> FlowRegistration {
    let node = NodeId::new("test.inc").unwrap();
    let seed = SlotRef::new(node.clone(), "in");
    let out = SlotRef::new(node, "out");
    FlowRegistration::new(
        body,
        revision,
        KindId::new("test.flow-registry.tool").unwrap(),
        "incrementer",
        "increments the integer at `in`",
    )
    .input_schema(serde_json::json!({
        "type": "object",
        "properties": {"value": {"type": "integer"}},
        "required": ["value"],
    }))
    .output_schema(serde_json::json!({"type": "integer"}))
    .with_default_adapters(seed, out)
}

#[tokio::test]
async fn register_resolve_returns_topology_and_terminals() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let flow_id = FlowId::new("test.flow-registry.resolve").unwrap();
    let revision = FlowRevisionId::new();

    registry
        .register(registration(fixture_body(&flow_id), revision), &kinds)
        .await
        .expect("register succeeds");

    let resolved = registry
        .resolve(&flow_id, &revision)
        .await
        .expect("resolve hits");

    // Topology has the one node we declared.
    let node = NodeId::new("test.inc").unwrap();
    assert!(resolved.topology.behaviors.contains_key(&node));
    assert!(resolved.topology.triggers.contains_key(&node));

    // The default-adapter convenience pushed `out` onto
    // terminals automatically.
    assert_eq!(resolved.terminal_slots.len(), 1);
    assert_eq!(resolved.terminal_slots[0].node, node);
    assert_eq!(resolved.terminal_slots[0].slot, "out");

    // Tool metadata round-trips.
    assert_eq!(resolved.name, "incrementer");
    assert_eq!(resolved.tool_id.as_str(), "test.flow-registry.tool");
}

#[tokio::test]
async fn resolve_unknown_pair_returns_not_found() {
    let registry = FlowRegistry::new();
    let flow_id = FlowId::new("test.flow-registry.missing").unwrap();
    let revision = FlowRevisionId::new();
    let err = registry
        .resolve(&flow_id, &revision)
        .await
        .expect_err("missing pair must error");
    assert!(matches!(err, FlowRegistryError::NotFound { .. }));
}

#[tokio::test]
async fn duplicate_revision_is_refused() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let flow_id = FlowId::new("test.flow-registry.dup").unwrap();
    let revision = FlowRevisionId::new();

    registry
        .register(registration(fixture_body(&flow_id), revision), &kinds)
        .await
        .expect("first register succeeds");
    let err = registry
        .register(registration(fixture_body(&flow_id), revision), &kinds)
        .await
        .expect_err("second register must fail");
    assert!(matches!(err, FlowRegistryError::DuplicateRevision { .. }));
}

#[tokio::test]
async fn unknown_terminal_node_is_caught_at_register_time() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let flow_id = FlowId::new("test.flow-registry.bad-terminal").unwrap();
    let revision = FlowRevisionId::new();

    let body = fixture_body(&flow_id);
    let bogus = SlotRef::new(NodeId::new("test.not-declared").unwrap(), "out");
    let spec = FlowRegistration::new(
        body,
        revision,
        KindId::new("test.flow-registry.bad").unwrap(),
        "bad",
        "bad",
    )
    .input_schema(serde_json::json!({"type": "object"}))
    .output_schema(serde_json::json!({"type": "object"}))
    .terminal_slots(vec![bogus])
    .with_adapters(
        Arc::new(|_| Vec::new()),
        Arc::new(|_| serde_json::Value::Null),
    );

    let err = registry
        .register(spec, &kinds)
        .await
        .expect_err("bad terminal node must fail");
    assert!(matches!(err, FlowRegistryError::UnknownTerminalNode { .. }));
}

#[tokio::test]
async fn from_registry_builds_tool_that_runs_end_to_end() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let flow_id = FlowId::new("test.flow-registry.run").unwrap();
    let revision = FlowRevisionId::new();
    let engine = make_engine();

    registry
        .register(registration(fixture_body(&flow_id), revision), &kinds)
        .await
        .expect("register");

    let tool = FlowAsTool::from_registry(&registry, &flow_id, &revision, engine.clone())
        .await
        .expect("from_registry");

    // Definition surfaces correctly (name + schemas round-trip).
    let def = tool.definition();
    assert_eq!(def.name, "incrementer");
    assert_eq!(def.input_schema["type"], "object");
    assert_eq!(tool.output_schema()["type"], "integer");

    // Invoke runs the flow once: input.value=7 → out=8.
    let out = tool
        .invoke(serde_json::json!({"value": 7}))
        .await
        .expect("invoke");
    assert_eq!(out, serde_json::json!(8));
}

#[tokio::test]
async fn register_yaml_loads_file_and_resolves() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("flow.yaml");
    let yaml = r#"
flow_id: test.flow-registry.yaml
nodes:
  - id: test.inc
    kind: test.flow-registry.increment
    triggers: ["in"]
links: []
"#;
    std::fs::write(&path, yaml).unwrap();

    let flow_id = FlowId::new("test.flow-registry.yaml").unwrap();
    let revision = FlowRevisionId::new();
    registry
        .register_yaml(&path, &kinds, |body| registration(body, revision))
        .await
        .expect("register_yaml");

    let resolved = registry
        .resolve(&flow_id, &revision)
        .await
        .expect("resolve");
    let node = NodeId::new("test.inc").unwrap();
    assert!(resolved.topology.behaviors.contains_key(&node));

    // And the loaded flow runs end-to-end via from_registry.
    let engine = make_engine();
    let tool = FlowAsTool::from_registry(&registry, &flow_id, &revision, engine)
        .await
        .expect("from_registry");
    let out = tool
        .invoke(serde_json::json!({"value": 41}))
        .await
        .expect("invoke");
    assert_eq!(out, serde_json::json!(42));
}

#[tokio::test]
async fn register_yaml_rejects_malformed_file() {
    let kinds = fixture_kinds().await;
    let registry = FlowRegistry::new();
    let dir = tempdir().unwrap();
    let path = dir.path().join("broken.yaml");
    std::fs::write(&path, "flow_id: not a string but ok\n: : : invalid").unwrap();
    let revision = FlowRevisionId::new();
    let err = registry
        .register_yaml(&path, &kinds, |body| registration(body, revision))
        .await
        .expect_err("malformed YAML must error");
    assert!(matches!(err, FlowRegistryError::YamlShape { .. }));
}
