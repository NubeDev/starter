//! The agent loop primitive.
//!
//! v0 shape: one user prompt in, one final reply out, with at most a
//! single round of tool dispatch in between. The runner is called
//! twice in the with-tools path: first to produce the tool-use, then
//! again with the tool results stitched into the conversation
//! history so the model can produce a final assistant reply that
//! consumes them.

use std::sync::Arc;

use starter_spi::ai::{
    AiRunner, Cancel, CliCfg, Provider, RestCfg, RunResult, RunnerInput, SessionId,
};
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::prompt::{assistant_tool_use_message, user_tool_results_message};
use crate::tool_set::ToolSet;

/// Single-turn agent loop with at most one round of tool dispatch.
pub struct AgentLoop {
    runner: Arc<dyn AiRunner>,
    tools: ToolSet,
}

impl AgentLoop {
    /// Build a loop bound to the supplied runner and tool set.
    pub fn new(runner: Arc<dyn AiRunner>, tools: ToolSet) -> Self {
        Self { runner, tools }
    }

    /// Drive the loop end-to-end and return the model's final reply.
    pub async fn run(&self, prompt: String) -> Result<String, AgentError> {
        let first = self.call(prompt.clone(), Vec::new()).await?;
        if first.tool_uses.is_empty() {
            return Ok(first.text);
        }

        let mut results = Vec::with_capacity(first.tool_uses.len());
        for call in &first.tool_uses {
            let tool = self
                .tools
                .get(&call.name)
                .ok_or_else(|| AgentError::UnknownTool(call.name.clone()))?;
            let out = tool
                .invoke(call.input.clone())
                .await
                .map_err(|e| AgentError::Tool {
                    name: call.name.clone(),
                    message: e.to_string(),
                })?;
            results.push((call.name.clone(), out));
        }

        let history = vec![
            assistant_tool_use_message(&first.text, &first.tool_uses),
            user_tool_results_message(&results),
        ];
        let second = self.call(prompt, history).await?;
        Ok(second.text)
    }

    async fn call(
        &self,
        prompt: String,
        history: Vec<starter_spi::ai::HistoryMessage>,
    ) -> Result<RunResult, AgentError> {
        let input = match self.runner.provider() {
            Provider::Claude | Provider::Codex | Provider::Copilot => {
                // CLI runners take a single combined prompt — they
                // do not consume `tools` or `history` directly. For
                // multi-turn, we serialise history into the prompt;
                // for tool dispatch through a CLI binary, the long-
                // term plan is an MCP-server bridge (see
                // LONG-TERM.md §"Tool-call streaming").
                let combined = if history.is_empty() {
                    prompt
                } else {
                    let mut s = String::new();
                    for m in &history {
                        s.push_str(&format!("[{}] {}\n", m.role, m.content));
                    }
                    s.push_str(&prompt);
                    s
                };
                RunnerInput::Cli(CliCfg {
                    prompt: combined,
                    ..Default::default()
                })
            }
            Provider::Anthropic | Provider::OpenAi => RunnerInput::Rest(RestCfg {
                prompt,
                history,
                tools: self.tools.definitions(),
                ..Default::default()
            }),
        };
        // The loop is non-streaming today; the channel is wired only
        // because the trait requires it. Capacity 16 swallows the
        // small bursts a runner emits without back-pressuring it.
        let (tx, _rx) = mpsc::channel(16);
        let cancel = NoopCancel;
        self.runner
            .run(input, SessionId::from("starter-ai-agent"), tx, &cancel)
            .await
            .map_err(|e| AgentError::Runner(e.to_string()))
    }
}

/// Always-open [`Cancel`] impl used by the loop until cooperative
/// cancellation lands (see LONG-TERM.md). The outer `tokio::time::timeout`
/// the caller wraps the loop in still applies.
struct NoopCancel;

impl Cancel for NoopCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}
