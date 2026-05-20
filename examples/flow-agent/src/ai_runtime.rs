//! Stage 4: agent chat runtime.
//!
//! Owns a shared [`starter_ai::Registry`] (built from
//! `Registry::with_defaults()`, so whichever provider features are
//! enabled at compile time light up automatically). Exposes two
//! affordances to the REST layer:
//!
//! 1. [`AiRuntime::list_providers`] — read-only detection so the
//!    Settings page can show why an agent might fail.
//! 2. [`AiRuntime::run_agent`] — turn an `Agent` row + a fresh chat
//!    turn into a stream of `axum::response::sse::Event` frames whose
//!    payload shape matches the default `createSseAdapter` parser in
//!    `@nube/starter-ui-chat`:
//!
//!    - `data: {"type":"text","text":"…"}` — assistant tokens.
//!    - `data: {"type":"tool-call","toolCall":{…}}` — tool invocations.
//!    - `data: {"type":"error","error":"…"}` — runner-level errors.
//!    - `data: [DONE]` — terminal frame (always emitted).
//!
//! Conversations are not persisted at this stage; the frontend keeps
//! a react-query-scoped history per page mount.

use std::sync::Arc;

use axum::response::sse;
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use utoipa::ToSchema;

use starter_ai::{Registry, TokenCancel};
use starter_spi::ai::{
    AiRunner, CliCfg, Event, EventKind, HistoryMessage, Provider, RestCfg, RunnerInput, SessionId,
};

use crate::domain::Agent;

/// What `GET /api/providers` returns.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProviderStatusDto {
    /// Stable provider id (e.g. `"claude"`, `"anthropic"`, `"openai"`).
    pub id: String,
    /// Human label rendered on the Settings page.
    pub label: String,
    /// `true` when the runner reports ready / env var is non-empty.
    pub available: bool,
    /// One-line hint shown when `available == false`.
    pub hint: String,
}

/// Errors `run_agent` can surface up the REST stack.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AgentRunError {
    /// Agent's `provider` field did not parse to a known `Provider`.
    #[error("unknown provider: `{0}`")]
    UnknownProvider(String),
    /// `Provider` is known but no runner is registered (feature off).
    #[error("provider `{0}` is not available in this build")]
    ProviderUnavailable(String),
}

/// Cheap-to-clone handle around the AI runner registry.
#[derive(Clone)]
pub struct AiRuntime {
    registry: Arc<Registry>,
}

impl AiRuntime {
    /// Build the runtime with every compiled-in provider registered.
    pub fn new() -> Self {
        Self {
            registry: Arc::new(Registry::with_defaults()),
        }
    }

    /// Probe each provider the Settings page cares about.
    pub async fn list_providers(&self) -> Vec<ProviderStatusDto> {
        let registered = self.registry.list().await;
        let claude_ready = registered
            .iter()
            .any(|p| matches!(p.provider, Provider::Claude) && p.available);

        let anthropic_key = std::env::var("ANTHROPIC_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);
        let openai_key = std::env::var("OPENAI_API_KEY")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false);

        vec![
            ProviderStatusDto {
                id: "claude".into(),
                label: "Claude CLI session".into(),
                available: claude_ready,
                hint: if claude_ready {
                    "Detected on PATH.".into()
                } else {
                    "Install Claude Code (`claude`) and run `claude auth login`.".into()
                },
            },
            ProviderStatusDto {
                id: "anthropic".into(),
                label: "ANTHROPIC_API_KEY".into(),
                available: anthropic_key,
                hint: if anthropic_key {
                    "Environment variable is set.".into()
                } else {
                    "Export `ANTHROPIC_API_KEY` to enable the Anthropic REST runner.".into()
                },
            },
            ProviderStatusDto {
                id: "openai".into(),
                label: "OPENAI_API_KEY".into(),
                available: openai_key,
                hint: if openai_key {
                    "Environment variable is set.".into()
                } else {
                    "Export `OPENAI_API_KEY` to enable the OpenAI runner.".into()
                },
            },
        ]
    }

    /// Resolve an agent's `provider` string to a [`Provider`] +
    /// registered runner.
    fn resolve(
        &self,
        provider: &str,
    ) -> Result<(Provider, Arc<dyn AiRunner>), AgentRunError> {
        let p = match provider {
            // The flow_engine module registers the Claude runner under
            // `anthropic.claude`; agents created via the UI use that
            // exact string. Accept the short alias too.
            "anthropic.claude" | "claude" => Provider::Claude,
            "anthropic" => Provider::Anthropic,
            "openai" => Provider::OpenAi,
            "codex" => Provider::Codex,
            "copilot" => Provider::Copilot,
            other => return Err(AgentRunError::UnknownProvider(other.to_owned())),
        };
        let runner = self
            .registry
            .get(&p)
            .ok_or_else(|| AgentRunError::ProviderUnavailable(p.to_string()))?;
        Ok((p, runner))
    }

    /// Drive one chat turn against the agent's provider and return a
    /// stream of pre-formatted SSE events.
    ///
    /// `prompt` is the user's freshly typed text; `history` carries
    /// prior turns the chat surface already accumulated (REST runners
    /// consume it directly; CLI runners fold it into the system
    /// prompt as plain text since `claude-wrapper` has no
    /// `--history` flag).
    pub fn run_agent(
        &self,
        agent: &Agent,
        prompt: String,
        history: Vec<HistoryMessage>,
    ) -> Result<
        impl Stream<Item = Result<sse::Event, std::convert::Infallible>> + Send + 'static,
        AgentRunError,
    > {
        let (provider, runner) = self.resolve(&agent.provider)?;
        let input = build_input(&provider, agent, prompt, history);
        let session_id = SessionId::from(format!("agent-{}", agent.id));

        let (tx, rx) = mpsc::channel::<Event>(32);

        // Spawn the runner. We don't expose cancellation to the chat
        // adapter yet — the SSE stream just runs until the runner
        // closes its `OnEvent` sender. The TokenCancel is held by the
        // task so the trait object satisfies the `&dyn Cancel`
        // lifetime requirement.
        let runner_handle = runner;
        tokio::spawn(async move {
            let cancel = TokenCancel::new();
            if let Err(err) = runner_handle.run(input, session_id, tx, &cancel).await {
                tracing::error!(error = %err, "agent runner failed");
            }
        });

        // Translate Event → SSE frame, then append the terminal
        // `[DONE]` sentinel so `createSseAdapter`'s default parser can
        // close cleanly.
        let event_stream = ReceiverStream::new(rx).filter_map(|ev| async move { event_to_sse(&ev) });
        let done = futures::stream::once(async {
            sse::Event::default().data("[DONE]")
        });
        let combined = event_stream
            .chain(done)
            .map(|e| Ok::<_, std::convert::Infallible>(e));
        Ok(combined)
    }
}

impl Default for AiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the runner input variant the resolved provider expects.
fn build_input(
    provider: &Provider,
    agent: &Agent,
    prompt: String,
    history: Vec<HistoryMessage>,
) -> RunnerInput {
    match provider {
        Provider::Claude | Provider::Codex | Provider::Copilot => {
            // CLI runners take a single prompt + optional system
            // context. Fold history into the system prompt so the
            // model still sees prior turns.
            let folded_history = fold_history_for_cli(&history);
            let system_prompt = match (agent.system_prompt.as_deref(), folded_history.as_str()) {
                (None, "") => None,
                (Some(s), "") => Some(s.to_owned()),
                (None, h) => Some(h.to_owned()),
                (Some(s), h) => Some(format!("{s}\n\n# Prior conversation\n{h}")),
            };
            RunnerInput::Cli(CliCfg {
                prompt,
                system_prompt,
                model: Some(agent.model.clone()),
                permission_mode: Some(starter_spi::ai::PermissionMode::Bypass),
                ..CliCfg::default()
            })
        }
        Provider::Anthropic | Provider::OpenAi => RunnerInput::Rest(RestCfg {
            prompt,
            system_prompt: agent.system_prompt.clone(),
            model: Some(agent.model.clone()),
            history,
            ..RestCfg::default()
        }),
    }
}

fn fold_history_for_cli(history: &[HistoryMessage]) -> String {
    history
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert an `Event` from the runner to the SSE frame shape the chat
/// adapter expects. Returns `None` for events the chat surface
/// doesn't render (e.g. `Connected`, `Done` — the latter is replaced
/// with the literal `[DONE]` sentinel by the caller).
fn event_to_sse(ev: &Event) -> Option<sse::Event> {
    let payload = match &ev.kind {
        EventKind::Text { content } => json!({ "type": "text", "text": content }),
        EventKind::ToolUse { id, name, input } => json!({
            "type": "tool-call",
            "toolCall": {
                "id": id.clone().unwrap_or_default(),
                "name": name,
                "args": input.clone().unwrap_or(serde_json::Value::Null),
                "state": "running",
            },
        }),
        EventKind::Error { message } => json!({ "type": "error", "error": message }),
        EventKind::Connected { .. } | EventKind::Done { .. } => return None,
    };
    Some(sse::Event::default().data(payload.to_string()))
}
