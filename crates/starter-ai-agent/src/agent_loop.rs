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
    AiRunner, Cancel, CliCfg, PermissionMode, Provider, RestCfg, RunResult, RunnerInput,
    SessionId,
};
use tokio::sync::mpsc;

use crate::error::AgentError;
use crate::prompt::{assistant_tool_use_message, user_tool_results_message};
use crate::tool_set::ToolSet;

/// Single-turn agent loop with at most one round of tool dispatch.
pub struct AgentLoop {
    runner: Arc<dyn AiRunner>,
    tools: ToolSet,
    /// Optional MCP-bridge URL forwarded into [`CliCfg::mcp_url`]
    /// when the runner is a CLI wrapper. When set the wrapped CLI
    /// process attaches to the named MCP server and the upstream
    /// model can dispatch *host* tools mid-turn — orthogonal to
    /// the locally-dispatched [`tools`] set the loop owns (which
    /// runs in this process and is what e.g. recorded fixtures
    /// exercise). `None` keeps the legacy "CLI without a tool
    /// catalogue" shape, which is what every unit/integration test
    /// in this workspace exercises today.
    mcp_url: Option<String>,
    /// Bearer token paired with [`Self::mcp_url`]. Only meaningful
    /// when `mcp_url` is `Some`.
    mcp_token: Option<String>,
    /// CLI tool-filter pattern forwarded as `--allowedTools` to
    /// the wrapped binary. Restricts which tools the model may
    /// call — including the CLI's *built-in* tools
    /// (`Bash`, `Read`, `AskUserQuestion`, …) and the MCP-
    /// bridged tools (`mcp__acme__*`). Empty / `None` keeps the
    /// CLI default, which permits the full built-in catalogue —
    /// suitable for ad-hoc Claude Code use, terrible for an
    /// assistant whose only job is to dispatch our MCP tools
    /// (the model will reach for `AskUserQuestion` instead of
    /// acting). No-op for non-CLI runners.
    allowed_tools: Option<String>,
    /// CLI permission mode forwarded to `CliCfg::permission_mode`.
    /// `None` keeps the CLI's interactive default — fatal for
    /// headless surfaces because every tool call stalls waiting
    /// for stdin approval. The chat surface sets this to
    /// [`PermissionMode::Bypass`] because the host has already
    /// authorised the principal at the HTTP boundary; the model
    /// merely actuates tools the operator implicitly approved by
    /// signing in.
    permission_mode: Option<PermissionMode>,
}

impl AgentLoop {
    /// Build a loop bound to the supplied runner and tool set.
    pub fn new(runner: Arc<dyn AiRunner>, tools: ToolSet) -> Self {
        Self {
            runner,
            tools,
            mcp_url: None,
            mcp_token: None,
            allowed_tools: None,
            permission_mode: None,
        }
    }

    /// Restrict the wrapped CLI to a single tool-filter pattern
    /// (e.g. `"mcp__acme__*"` to allow only MCP-bridged tools
    /// and disable every built-in including `AskUserQuestion`).
    /// Empty / whitespace-only is treated as unset.
    pub fn with_allowed_tools(mut self, pattern: Option<String>) -> Self {
        self.allowed_tools = pattern.filter(|s| !s.trim().is_empty());
        self
    }

    /// Override the wrapped CLI's permission mode. The chat
    /// surface uses [`PermissionMode::Bypass`] because the host
    /// has already gated the request at the HTTP layer; without
    /// this the CLI defaults to interactive approval and every
    /// tool call stalls.
    pub fn with_permission_mode(mut self, mode: Option<PermissionMode>) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Attach an MCP bridge to every CLI runner call this loop
    /// makes. Empty strings are treated as "unset" so callers can
    /// pass raw env-var reads without filtering. No-op for non-CLI
    /// runners — REST providers carry their own tool list via
    /// [`RestCfg::tools`].
    pub fn with_mcp(
        mut self,
        url: Option<String>,
        token: Option<String>,
    ) -> Self {
        self.mcp_url = url.filter(|s| !s.trim().is_empty());
        self.mcp_token = token.filter(|s| !s.trim().is_empty());
        self
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
                    mcp_url: self.mcp_url.clone(),
                    mcp_token: self.mcp_token.clone(),
                    allowed_tools: self.allowed_tools.clone(),
                    permission_mode: self.permission_mode,
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
