//! Runner asks for a tool the loop's `ToolSet` does not carry —
//! the loop surfaces `AgentError::UnknownTool`.

use std::sync::Arc;

use serde_json::json;
use starter_ai_agent::testing::MockAiRunner;
use starter_ai_agent::{AgentError, AgentLoop, ToolSet};
use starter_spi::ai::{RunResult, ToolUse};

#[tokio::test]
async fn unknown_tool_surfaces_typed_error() {
    let runner = Arc::new(MockAiRunner::new(vec![RunResult {
        text: String::new(),
        tool_uses: vec![ToolUse {
            id: "call-1".to_owned(),
            name: "does-not-exist".to_owned(),
            input: json!({}),
        }],
        ..Default::default()
    }]));
    let agent = AgentLoop::new(runner, ToolSet::default());
    let err = agent.run("hi".to_owned()).await.expect_err("must fail");
    match err {
        AgentError::UnknownTool(name) => assert_eq!(name, "does-not-exist"),
        other => panic!("expected UnknownTool, got {other:?}"),
    }
}
