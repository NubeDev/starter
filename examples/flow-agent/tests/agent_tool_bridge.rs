//! Stage 5 — agent-as-tool bridge smoke.
//!
//! Wires a `RecordingAiRunner` into a flow-agent `AiRuntime`, creates
//! a `trigger.explicit → log` flow plus an agent with
//! `tools=["flow:<id>"]`, then asserts that:
//!
//! 1. The synthesised tool set advertised to the runner contains the
//!    flow's `flow:<id>` entry (`tools_count >= 1` on turn 0).
//! 2. When the script emits a tool call against that name, the
//!    runtime fires the flow through `FlowEngine`, persists a run row,
//!    and surfaces a `tool-result` SSE frame whose payload contains
//!    the log node's `emitted` slot value.
//! 3. The agent receives the log output as a `user`-role history
//!    message on the second turn (so it could act on it).

use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use serde_json::{json, Value};
use tokio::time::timeout;

use starter_ai::testing::{RecordingAiRunner, ScriptTurn};
use starter_ai::Registry as AiRegistry;
use starter_spi::ai::{Provider, ToolUse};
use starter_store_sqlite::{migrate, pool};

use flow_agent::ai_runtime::AiRuntime;
use flow_agent::domain::{CreateAgent, CreateFlow};
use flow_agent::flow_engine::FlowEngine;
use flow_agent::migrations;
use flow_agent::sse::EventHub;
use flow_agent::store::{AgentStore, FlowStore, RunStore};

/// Cap the SSE drain so a stuck stream fails the test instead of
/// hanging the whole `cargo test` invocation.
const DRAIN_TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn agent_invokes_flow_tool_and_receives_log_output() {
    // -----------------------------------------------------------
    // 1. Stand up an isolated in-memory backend.
    // -----------------------------------------------------------
    let pool = pool::connect("sqlite::memory:").await.expect("connect");
    let mut chain = migrate(&pool);
    for src in migrations::sources() {
        chain = chain.with_source(src);
    }
    chain.run().await.expect("migrate");

    let sqlx = pool.sqlx().clone();
    let flows = Arc::new(FlowStore::new(sqlx.clone()));
    let agents = Arc::new(AgentStore::new(sqlx.clone()));
    let runs = Arc::new(RunStore::new(sqlx));
    let hub = Arc::new(EventHub::new());
    // 200 ms quiescence keeps the bridge's `trigger → log` run
    // terminating in well under the test timeout; the production
    // default (60 s) is sized for slow CLI invocations the test
    // doesn't exercise.
    let engine = FlowEngine::new().with_quiescence(Duration::from_millis(200));

    // -----------------------------------------------------------
    // 2. Create a `trigger → log` flow.
    //    The UI's edge slot names follow the flow_engine mapping:
    //    `fire` → `payload` (out) and `value` (in).
    // -----------------------------------------------------------
    let graph = json!({
        "nodes": [
            { "id": "trigger-1", "kind": "trigger" },
            { "id": "log-1",     "kind": "log" }
        ],
        "edges": [
            {
                "id": "e1",
                "source": "trigger-1",
                "sourceSlot": "fire",
                "target": "log-1",
                "targetSlot": "value"
            }
        ]
    });
    let flow = flows
        .create(CreateFlow {
            name: "echo".into(),
            description: Some("echo input to the log".into()),
            graph: Some(graph),
        })
        .await
        .expect("create flow");
    let flow_tool_name = format!("flow:{}", flow.id);

    // -----------------------------------------------------------
    // 3. Script: turn 0 calls the flow tool; turn 1 acknowledges.
    //    The `RecordingAiRunner` defaults to Provider::Anthropic so
    //    the runtime's REST loop path picks it up directly.
    // -----------------------------------------------------------
    let script = vec![
        ScriptTurn {
            text: "calling flow…".into(),
            tool_uses: vec![ToolUse {
                id: "call-0".into(),
                name: flow_tool_name.clone(),
                input: json!("hello"),
            }],
        },
        ScriptTurn::text("done"),
    ];
    let recording = RecordingAiRunner::new(script).with_provider(Provider::Anthropic);
    let ai_registry = AiRegistry::new();
    ai_registry.register(recording.clone());
    let ai = AiRuntime::with_registry(
        Arc::new(ai_registry),
        flows.clone(),
        engine.clone(),
        runs.clone(),
        hub.clone(),
    );

    // -----------------------------------------------------------
    // 4. Create the agent and run a chat turn through the runtime.
    // -----------------------------------------------------------
    let agent = agents
        .create(CreateAgent {
            name: "bridge-bot".into(),
            provider: "anthropic".into(),
            model: "claude-3-haiku-20240307".into(),
            system_prompt: None,
            tools: vec![flow_tool_name.clone()],
        })
        .await
        .expect("create agent");

    let stream = ai
        .run_agent_raw(&agent, "run my flow with 'hello'".into(), Vec::new())
        .expect("run_agent_raw");

    // Drain SSE frames until [DONE] or timeout.
    let frames: Vec<Value> = timeout(DRAIN_TIMEOUT, async {
        let mut out = Vec::new();
        let mut stream = std::pin::pin!(stream);
        while let Some(data) = stream.next().await {
            if data == "[DONE]" {
                break;
            }
            let parsed: Value = serde_json::from_str(&data)
                .unwrap_or_else(|e| panic!("bad SSE payload `{data}`: {e}"));
            out.push(parsed);
        }
        out
    })
    .await
    .expect("stream drained within timeout");

    // -----------------------------------------------------------
    // 5. Frame-level assertions.
    // -----------------------------------------------------------
    let tool_calls: Vec<&Value> = frames
        .iter()
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("tool-call"))
        .collect();
    let tool_results: Vec<&Value> = frames
        .iter()
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("tool-result"))
        .collect();

    // The runtime emits at least one `tool-call` frame (its own
    // host-side dispatch event); the recording runner also emits a
    // ToolUse event but it only surfaces over the streaming `Event`
    // channel, which `RecordingAiRunner` does not push to. Either way
    // we expect ≥ 1 here for the host dispatch frame.
    assert!(
        !tool_calls.is_empty(),
        "expected a tool-call SSE frame, got {frames:#?}"
    );
    let hit = tool_calls
        .iter()
        .any(|v| v["toolCall"]["name"].as_str() == Some(flow_tool_name.as_str()));
    assert!(hit, "no tool-call frame targeted `{flow_tool_name}`");

    assert_eq!(
        tool_results.len(),
        1,
        "expected exactly one tool-result frame, got {tool_results:#?}"
    );
    let tr = tool_results[0];
    assert_eq!(
        tr["toolCall"]["name"].as_str(),
        Some(flow_tool_name.as_str())
    );
    assert_eq!(tr["toolCall"]["state"].as_str(), Some("done"));
    let result_str = tr["toolCall"]["result"]
        .as_str()
        .expect("tool-result.result is a string");
    let result_json: Value =
        serde_json::from_str(result_str).expect("tool-result.result parses as JSON");
    // The log node passes its `value` through to `emitted`. We sent
    // `"hello"` as the tool input, so the terminal output should
    // carry that string under some `<node_id>.emitted` key. The
    // engine's `RunCompleted.output` map keys terminal slots as
    // `"{node_id}.{slot_name}"`.
    let obj = result_json
        .as_object()
        .expect("tool result JSON is an object");
    let emitted = obj
        .iter()
        .find_map(|(k, v)| k.ends_with(".emitted").then_some(v))
        .expect("tool result map carries an `.emitted` terminal slot");
    assert_eq!(
        emitted,
        &json!("hello"),
        "tool result `.emitted` did not match the fired payload: {result_json}"
    );

    // -----------------------------------------------------------
    // 6. Behaviour assertions on the recording runner.
    //    Turn 0 must advertise our synthesised flow tool, and turn 1
    //    must include the dispatched tool result in history (added
    //    by the loop as a `user`-role message).
    // -----------------------------------------------------------
    let calls = recording.calls();
    assert_eq!(calls.len(), 2, "expected two runner turns, got {calls:?}");
    assert!(
        calls[0].tools_count >= 1,
        "turn 0 should advertise ≥ 1 synthesised flow tool (got {})",
        calls[0].tools_count
    );
    // Turn 1's history = prior user prompt + assistant text + dispatched
    // tool reply = 3 entries.
    assert!(
        calls[1].history_len >= 3,
        "turn 1 history should include the dispatched tool reply (got {})",
        calls[1].history_len
    );

    // -----------------------------------------------------------
    // 7. A `runs` row should exist for the flow (the agent fired it).
    // -----------------------------------------------------------
    let flow_runs = runs.list_for_flow(&flow.id).await.expect("list runs");
    assert_eq!(
        flow_runs.len(),
        1,
        "expected one recorded run, got {flow_runs:?}"
    );
    assert_eq!(flow_runs[0].status, "ok");
}
