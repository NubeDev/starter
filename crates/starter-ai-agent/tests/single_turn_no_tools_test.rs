//! Runner returns a final reply directly — no tool dispatch needed.

use std::sync::Arc;

use starter_ai_agent::testing::MockAiRunner;
use starter_ai_agent::{AgentLoop, ToolSet};
use starter_spi::ai::RunResult;

#[tokio::test]
async fn single_turn_no_tools_returns_runner_text() {
    let runner = Arc::new(MockAiRunner::new(vec![RunResult {
        text: "hello world".to_owned(),
        ..Default::default()
    }]));
    let agent = AgentLoop::new(runner, ToolSet::default());
    let outcome = agent.run("hi".to_owned()).await.expect("loop succeeds");
    assert_eq!(outcome.text, "hello world");
}
