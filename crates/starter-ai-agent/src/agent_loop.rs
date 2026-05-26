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
    AiRunner, Cancel, CliCfg, Event, PermissionMode, Provider, RestCfg, RunResult, RunnerInput,
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
    /// bridged tools (`mcp__rubix__*`). Empty / `None` keeps the
    /// CLI default, which permits the full built-in catalogue —
    /// suitable for ad-hoc Claude Code use, terrible for an
    /// assistant whose only job is to dispatch our MCP tools
    /// (the model will reach for `AskUserQuestion` instead of
    /// acting). No-op for non-CLI runners.
    allowed_tools: Option<String>,
    /// CLI built-in tool restriction forwarded to `CliCfg::tools`.
    /// Distinct from [`Self::allowed_tools`]: `--tools` controls the
    /// CLI's *built-in* catalogue (`Bash`, `Read`, `Edit`, …),
    /// `--allowedTools` gates the MCP-bridged tools. `Some("")`
    /// (empty list) means "no built-ins" — used by `ai-agent` flow
    /// nodes that should only reach MCP. `None` keeps the CLI
    /// default, which permits the full built-in catalogue.
    cli_tools: Option<String>,
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
            cli_tools: None,
            permission_mode: None,
        }
    }

    /// Restrict the wrapped CLI's *built-in* tool catalogue. Pass
    /// `Some(String::new())` to disable every built-in (the
    /// `tools: []` shape from rubix flow YAML — the agent's only
    /// reachable surface is the MCP bridge). `None` keeps the CLI
    /// default, which permits the full built-in set. No-op for
    /// non-CLI runners.
    pub fn with_cli_tools(mut self, tools: Option<String>) -> Self {
        self.cli_tools = tools;
        self
    }

    /// Restrict the wrapped CLI to a single tool-filter pattern
    /// (e.g. `"mcp__rubix__*"` to allow only MCP-bridged tools
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
    pub fn with_mcp(mut self, url: Option<String>, token: Option<String>) -> Self {
        self.mcp_url = url.filter(|s| !s.trim().is_empty());
        self.mcp_token = token.filter(|s| !s.trim().is_empty());
        self
    }

    /// Drive the loop end-to-end and return the model's final reply
    /// alongside every streamed [`Event`] the runner emitted.
    ///
    /// Thin wrapper around [`Self::run_with_outcome`] that preserves
    /// the original `Result` shape for callers that don't need the
    /// "events collected up to the point of failure" surface
    /// (e.g. the 3-test suite in this crate, the
    /// `starter-flow-node-loop` body). On `Err` the events from the
    /// partial run are dropped on the floor; callers that want them
    /// (today: the rubix `ai-agent` flow node feeding the dashboard
    /// editor SSE channel) should use [`Self::run_with_outcome`]
    /// directly.
    pub async fn run(&self, prompt: String) -> Result<RunOutcome, AgentError> {
        let outcome = self.run_with_outcome(prompt).await;
        match outcome.error {
            Some(err) => Err(err),
            None => Ok(RunOutcome {
                text: outcome.text,
                events: outcome.events,
                error: None,
            }),
        }
    }

    /// Always returns a populated [`RunOutcome`] — events collected
    /// up to the point of failure are preserved on `outcome.error =
    /// Some(_)`, so a surface that wants to render per-step activity
    /// (e.g. live "AI is editing this page" UX for scope 11 §B7) can
    /// always show the `Text` / `ToolUse` history regardless of
    /// whether the run terminated cleanly.
    ///
    /// The internal control flow mirrors [`Self::run`] but each
    /// failure point returns a partial outcome instead of `?`-ing
    /// out: events accumulated during the first runner call survive
    /// a subsequent `UnknownTool` / `Tool` / second-call failure.
    pub async fn run_with_outcome(&self, prompt: String) -> RunOutcome {
        let mut events: Vec<Event> = Vec::new();
        let (first_res, mut first_events) = self.call(prompt.clone(), Vec::new()).await;
        events.append(&mut first_events);
        let first = match first_res {
            Ok(rr) => rr,
            Err(err) => {
                return RunOutcome {
                    text: String::new(),
                    events,
                    error: Some(err),
                }
            }
        };
        if first.tool_uses.is_empty() {
            return RunOutcome {
                text: first.text,
                events,
                error: None,
            };
        }

        let mut results = Vec::with_capacity(first.tool_uses.len());
        for call in &first.tool_uses {
            let tool = match self.tools.get(&call.name) {
                Some(t) => t,
                None => {
                    return RunOutcome {
                        text: first.text,
                        events,
                        error: Some(AgentError::UnknownTool(call.name.clone())),
                    }
                }
            };
            let out = match tool.invoke(call.input.clone()).await {
                Ok(v) => v,
                Err(e) => {
                    return RunOutcome {
                        text: first.text,
                        events,
                        error: Some(AgentError::Tool {
                            name: call.name.clone(),
                            message: e.to_string(),
                        }),
                    }
                }
            };
            results.push((call.name.clone(), out));
        }

        let history = vec![
            assistant_tool_use_message(&first.text, &first.tool_uses),
            user_tool_results_message(&results),
        ];
        let (second_res, mut second_events) = self.call(prompt, history).await;
        events.append(&mut second_events);
        let second = match second_res {
            Ok(rr) => rr,
            Err(err) => {
                return RunOutcome {
                    text: first.text,
                    events,
                    error: Some(err),
                }
            }
        };
        RunOutcome {
            text: second.text,
            events,
            error: None,
        }
    }

    /// Internal — drives one runner call and returns events
    /// unconditionally alongside either the `RunResult` (Ok) or the
    /// `AgentError` (Err). The previous `Result<(RunResult,
    /// Vec<Event>), AgentError>` shape silently dropped events on
    /// the Err branch because the early `?` skipped the collector
    /// `.await`. Surfacing both lets `run_with_outcome` honour its
    /// "events survive errors" contract.
    async fn call(
        &self,
        prompt: String,
        history: Vec<starter_spi::ai::HistoryMessage>,
    ) -> (Result<RunResult, AgentError>, Vec<Event>) {
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
                    tools: self.cli_tools.clone(),
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
        // Capacity 16 swallows the small bursts a runner emits
        // without back-pressuring it. The receiver runs in a spawned
        // task that drains every event into a `Vec` for the caller —
        // until this change the receiver was named `_rx` and the
        // events were silently discarded, which made the
        // `/api/v1/flows/{id}/run` response a single concatenated
        // `text` string with no visibility into per-step `Text` /
        // `ToolUse` activity (see
        // `rubix/docs/sessions/data-flow/2026-05-26-data-flow-07-agent-event-projection.md`).
        let (tx, mut rx) = mpsc::channel::<Event>(16);
        let collector = tokio::spawn(async move {
            let mut out = Vec::new();
            while let Some(ev) = rx.recv().await {
                out.push(ev);
            }
            out
        });
        let cancel = NoopCancel;
        let run_res = self
            .runner
            .run(input, SessionId::from("starter-ai-agent"), tx, &cancel)
            .await;
        // **Always** await the collector — `run_res` can be `Err`,
        // and in that case the events the runner emitted before
        // failing are exactly what live-feedback surfaces need. The
        // previous shape used `?` on `run_res` which short-circuited
        // past the `.await` and lost the partial event stream. `tx`
        // was moved into `runner.run` and dropped on return either
        // way, so the collector sees channel close in both paths.
        let events = collector.await.unwrap_or_default();
        match run_res {
            Ok(result) => (Ok(result), events),
            Err(e) => (Err(AgentError::Runner(e.to_string())), events),
        }
    }
}

/// Successful outcome of [`AgentLoop::run`].
///
/// Carries both the final assistant text and the ordered stream of
/// runner [`Event`]s observed during the run (across every runner
/// call the loop made — initial + post-tool-dispatch second call
/// in the with-tools path). Consumers that only need the text can
/// take `outcome.text`; consumers that want to render per-step
/// activity (e.g. the `rubix-agent` SSE surface for the dashboard
/// editor) read `outcome.events`.
#[derive(Debug, Clone)]
pub struct RunOutcome {
    /// Concatenated final assistant text. Empty string if the run
    /// failed before the first runner call produced any text.
    pub text: String,
    /// Ordered stream of runner events. May include
    /// [`EventKind::Connected`], [`EventKind::Text`],
    /// [`EventKind::ToolUse`], [`EventKind::Done`], and
    /// [`EventKind::Error`] in arrival order.
    ///
    /// [`EventKind::Connected`]: starter_spi::ai::EventKind::Connected
    /// [`EventKind::Text`]: starter_spi::ai::EventKind::Text
    /// [`EventKind::ToolUse`]: starter_spi::ai::EventKind::ToolUse
    /// [`EventKind::Done`]: starter_spi::ai::EventKind::Done
    /// [`EventKind::Error`]: starter_spi::ai::EventKind::Error
    pub events: Vec<Event>,
    /// `Some(_)` when the run terminated abnormally (runner failure,
    /// unknown tool the model invented, tool dispatch error, …).
    /// Events collected up to the point of failure remain in
    /// [`Self::events`] — this is the live-feedback contract
    /// surfaces like the rubix `ai-agent` flow node depend on so the
    /// dashboard editor can still render "AI got 3 chunks and then
    /// errored" rather than silently going dark.
    pub error: Option<crate::AgentError>,
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
