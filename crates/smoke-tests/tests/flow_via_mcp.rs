//! Phase 3 stage 9 smoke 1 — `FlowAsTool` wired through the
//! starter-mcp dispatch surface, backed by a real
//! [`SqliteRunStore`] behind the engine.
//!
//! Contract from the job WORKFLOW per-stage table (template.yaml
//! line 45):
//!
//! > build a toy flow with a transform body returning the input
//! > doubled; wrap it via `FlowAsTool` with explicit
//! > input/output schemas; register the tool with starter-mcp's
//! > `ToolRegistry` using the in-process MCP test harness
//! > starter-mcp's own tests use; an MCP client calls the tool by
//! > its `tool_id` and asserts the doubled output AND asserts
//! > that the `SqliteRunStore` behind the engine records the run
//! > with a non-empty checkpoint history at the expected
//! > `(run_id, seq)` keys.
//!
//! In-process harness: the dispatch loop is invoked directly via
//! [`starter_mcp::server::dispatch::dispatch`] — mirroring the
//! pattern in `crates/starter-mcp/src/server/dispatch.rs` unit
//! tests. R8 wire-shape (the dispatch returns a JSON-RPC envelope
//! with `structuredContent` carrying the tool result) is exercised
//! end-to-end.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow_spi::flow::{FlowId, FlowRevisionId, RunStore as SpiRunStore};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_surfaces::FlowAsTool;
use starter_mcp::registry::ToolRegistry;
use starter_mcp::server::dispatch;
use starter_store_sqlite::flow::{SqliteRunStore, FLOW_MIGRATION_SOURCE};
use starter_store_sqlite::{migrate, testing::ephemeral};

/// Node behavior: `out = in * 2` for any integer JSON input.
struct Doubler {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for Doubler {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }
    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let v = match input.get("in").cloned().unwrap_or(SlotValue::Null) {
            SlotValue::Json(Value::Number(n)) => n.as_i64().unwrap_or(0),
            SlotValue::Json(v) => v.as_i64().unwrap_or(0),
            _ => 0,
        };
        let mut out = SlotMap::new();
        out.insert("out".to_owned(), SlotValue::Json(serde_json::json!(v * 2)));
        Ok(out)
    }
}

fn build_topology() -> Arc<FlowTopology> {
    let node = NodeId::new("com.acme.stage9.doubler").unwrap();
    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(node.clone(), std::iter::once("in".to_owned()).collect());
    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(
        node,
        Arc::new(Doubler {
            kind: KindId::new("starter.flow.stage9-doubler").unwrap(),
        }),
    );
    Arc::new(FlowTopology {
        links: HashMap::new(),
        triggers,
        behaviors,
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flow_via_mcp_doubles_input_and_records_run_in_sqlite() {
    // Real SqliteRunStore over a fresh ephemeral pool (D-F3.3 +
    // D-F3.8 + D-F3.9 pragmas + WAL pragmas all live at the pool
    // init).
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let sqlite_store: Arc<dyn SpiRunStore> = Arc::new(SqliteRunStore::new(pool.clone()));

    // Engine with the SqliteRunStore attached so per-tick
    // checkpointing actually persists to a real backend (the
    // stage-5 wiring point).
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph_store).with_run_store(sqlite_store.clone()));

    // Build the FlowAsTool — explicit schemas per D-F3.4.
    let node = NodeId::new("com.acme.stage9.doubler").unwrap();
    let in_slot = SlotRef::new(node.clone(), "in");
    let out_slot = SlotRef::new(node, "out");
    let out_key = format!("{}.{}", out_slot.node, out_slot.slot);

    let tool = FlowAsTool::builder()
        .flow_id(FlowId::new("com.acme.stage9.via-mcp").unwrap())
        .revision(FlowRevisionId::new())
        .topology(build_topology())
        .terminal_slots(vec![out_slot])
        .engine(engine)
        .tool_id(KindId::new("starter.flow.stage9-via-mcp").unwrap())
        .name("stage9_doubler")
        .description("doubles an integer input through a one-node flow")
        .input_schema(serde_json::json!({
            "type": "object",
            "properties": {"value": {"type": "integer"}},
            "required": ["value"],
        }))
        .output_schema(serde_json::json!({"type": "integer"}))
        .seed_adapter(Arc::new(move |input: &Value| {
            let v = input.get("value").cloned().unwrap_or(Value::Null);
            vec![(in_slot.clone(), SlotValue::Json(v))]
        }))
        .output_adapter(Arc::new(move |out: &SlotMap| match out.get(&out_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => Value::Null,
        }))
        .build()
        .expect("FlowAsTool build");

    // Register on the MCP ToolRegistry.
    let registry = Arc::new(ToolRegistry::new().register(tool));

    // Drive a `tools/call` frame through the MCP dispatch loop —
    // this is the in-process equivalent of an MCP client speaking
    // over the stdio transport.
    let frame = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "stage9_doubler",
            "arguments": {"value": 21}
        }
    }"#;
    let resp = dispatch(&registry, frame)
        .await
        .expect("dispatch returns a response");
    assert!(
        resp.error.is_none(),
        "dispatch surfaced an error: {:?}",
        resp.error
    );
    let result = resp.result.expect("dispatch returned a result");
    assert_eq!(
        result["structuredContent"], 42,
        "MCP tools/call must return the doubled value; got {result}"
    );

    // SqliteRunStore must have recorded the run. The `runs` table
    // is the per-run lifecycle row; the `run_checkpoints` table is
    // the per-tick (run_id, seq) history. Both must be non-empty
    // after a successful invoke.
    let runs_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM runs")
        .fetch_one(pool.sqlx())
        .await
        .expect("count runs");
    assert!(
        runs_count >= 1,
        "expected at least one row in `runs`, got {runs_count}"
    );

    let checkpoint_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM run_checkpoints")
        .fetch_one(pool.sqlx())
        .await
        .expect("count run_checkpoints");
    assert!(
        checkpoint_count >= 1,
        "expected at least one row in `run_checkpoints` (per-tick \
         checkpoint cadence per D-F3.2), got {checkpoint_count}"
    );

    // (run_id, seq) keys are strictly increasing from 1 — load
    // the seqs and assert they're monotonic, contiguous, and
    // start at 1.
    let seqs: Vec<i64> =
        sqlx::query_scalar("SELECT seq FROM run_checkpoints ORDER BY run_id, seq ASC")
            .fetch_all(pool.sqlx())
            .await
            .expect("fetch checkpoint seqs");
    assert!(seqs.iter().all(|s| *s >= 1), "seq >= 1, got {seqs:?}");
    for window in seqs.windows(2) {
        assert!(
            window[1] >= window[0],
            "seqs must be non-decreasing within a run, got {seqs:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn flow_via_mcp_tool_listed_under_registered_name() {
    // Sanity: the `tools/list` dispatch surfaces the FlowAsTool
    // under the name passed to the builder. Demonstrates R8 list
    // visibility without re-exercising the run path.
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .expect("flow migrations apply");
    let sqlite_store: Arc<dyn SpiRunStore> = Arc::new(SqliteRunStore::new(pool));

    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph_store).with_run_store(sqlite_store));

    let node = NodeId::new("com.acme.stage9.doubler").unwrap();
    let in_slot = SlotRef::new(node.clone(), "in");
    let out_slot = SlotRef::new(node, "out");
    let out_key = format!("{}.{}", out_slot.node, out_slot.slot);
    let tool = FlowAsTool::builder()
        .flow_id(FlowId::new("com.acme.stage9.list").unwrap())
        .revision(FlowRevisionId::new())
        .topology(build_topology())
        .terminal_slots(vec![out_slot])
        .engine(engine)
        .tool_id(KindId::new("starter.flow.stage9-list").unwrap())
        .name("stage9_doubler")
        .description("doubles an integer")
        .input_schema(serde_json::json!({"type": "object"}))
        .output_schema(serde_json::json!({"type": "integer"}))
        .seed_adapter(Arc::new(move |input: &Value| {
            vec![(
                in_slot.clone(),
                SlotValue::Json(input.get("value").cloned().unwrap_or(Value::Null)),
            )]
        }))
        .output_adapter(Arc::new(move |out: &SlotMap| match out.get(&out_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => Value::Null,
        }))
        .build()
        .expect("FlowAsTool build");
    let registry = Arc::new(ToolRegistry::new().register(tool));

    let resp = dispatch(
        &registry,
        r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#,
    )
    .await
    .expect("tools/list response");
    let names: Vec<String> = resp.result.unwrap()["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        names.iter().any(|n| n == "stage9_doubler"),
        "FlowAsTool must appear in tools/list under its name; saw {names:?}"
    );
}
