//! Runner asks for one tool, mock tool returns a result, second
//! runner call returns the final reply that consumed it.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_ai_agent::testing::MockAiRunner;
use starter_ai_agent::{AgentLoop, ToolSet};
use starter_spi::ai::{RunResult, ToolUse};
use starter_spi::error::Result as SpiResult;
use starter_spi::tool::{Tool, ToolDefinition};

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "echo".to_owned(),
            description: "echo the input value".to_owned(),
            input_schema: json!({"type": "object"}),
        }
    }
    async fn invoke(&self, input: Value) -> SpiResult<Value> {
        Ok(json!({ "echoed": input }))
    }
}

#[tokio::test]
async fn tool_call_round_trip_yields_final_reply() {
    let runner = Arc::new(MockAiRunner::new(vec![
        // First call — model asks for the echo tool.
        RunResult {
            text: String::new(),
            tool_uses: vec![ToolUse {
                id: "call-1".to_owned(),
                name: "echo".to_owned(),
                input: json!({"hello": 1}),
            }],
            ..Default::default()
        },
        // Second call — model produces the final reply.
        RunResult {
            text: "done".to_owned(),
            ..Default::default()
        },
    ]));
    let tools = ToolSet::new(vec![Arc::new(EchoTool)]);
    let agent = AgentLoop::new(runner, tools);
    let reply = agent.run("call echo".to_owned()).await.expect("loop ok");
    assert_eq!(reply, "done");
}
