//! Agent chat runtime.
//!
//! Owns a shared [`starter_ai::Registry`] (built from
//! `Registry::with_defaults()`, so whichever provider features are
//! enabled at compile time light up automatically) plus host-side
//! handles (`FlowStore`, `FlowEngine`, `RunStore`, `EventHub`) so a
//! single chat turn can fan out into flow runs and back without
//! leaving the host process.
//!
//! Exposes three affordances to the REST layer:
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
//!    - `data: {"type":"tool-result","toolCall":{…}}` — flow-tool
//!      replies (host-emitted, used by the agent-as-tool bridge).
//!    - `data: {"type":"error","error":"…"}` — runner-level errors.
//!    - `data: [DONE]` — terminal frame (always emitted).
//! 3. [`AiRuntime::run_agent_raw`] — same as `run_agent` but yields
//!    the raw `data:` payload strings; the production SSE handler
//!    wraps them, the bridge integration test consumes them directly.
//!
//! The agent-as-tool bridge (flow-tool synthesis, dispatch, run
//! draining) lives next door in [`crate::agent_bridge`] so this file
//! stays under the 400-line workspace rule.
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

use starter_ai::Registry;
use starter_spi::ai::{AiRunner, HistoryMessage, Provider};

use crate::domain::Agent;
use crate::flow_engine::FlowEngine;
use crate::sse::EventHub;
use crate::store::{FlowStore, RunStore};

/// Hard ceiling on the in-runtime agentic loop. Mirrors the
/// `starter-flow-nodes::ai_agent::MAX_TURNS` posture: when the model
/// keeps emitting tool calls we eventually stop dispatching to keep
/// runaway agents bounded.
pub const MAX_AGENT_TURNS: u32 = 16;

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
    /// Could not enumerate flows for tool synthesis.
    #[error("flow registry error: {0}")]
    Registry(String),
}

/// Cheap-to-clone handle around the AI runner registry plus the host
/// surfaces the agent-as-tool bridge needs to fire flows.
#[derive(Clone)]
pub struct AiRuntime {
    registry: Arc<Registry>,
    flows: Arc<FlowStore>,
    engine: FlowEngine,
    runs: Arc<RunStore>,
    hub: Arc<EventHub>,
}

impl AiRuntime {
    /// Build the runtime with every compiled-in provider registered.
    pub fn new(
        flows: Arc<FlowStore>,
        engine: FlowEngine,
        runs: Arc<RunStore>,
        hub: Arc<EventHub>,
    ) -> Self {
        Self::with_registry(Arc::new(Registry::with_defaults()), flows, engine, runs, hub)
    }

    /// Construct with a caller-supplied registry. Used by integration
    /// tests to inject a `RecordingAiRunner` without spinning real
    /// provider impls.
    pub fn with_registry(
        registry: Arc<Registry>,
        flows: Arc<FlowStore>,
        engine: FlowEngine,
        runs: Arc<RunStore>,
        hub: Arc<EventHub>,
    ) -> Self {
        Self {
            registry,
            flows,
            engine,
            runs,
            hub,
        }
    }

    // -----------------------------------------------------------------
    // Bridge accessors. Kept `pub(crate)` so `agent_bridge.rs` can
    // reach the same handles without making the fields themselves
    // crate-public (preserves the current encapsulation of the
    // constructor surface).
    // -----------------------------------------------------------------

    pub(crate) fn flows(&self) -> &FlowStore {
        &self.flows
    }
    pub(crate) fn engine(&self) -> &FlowEngine {
        &self.engine
    }
    pub(crate) fn runs_store(&self) -> &RunStore {
        &self.runs
    }
    pub(crate) fn hub(&self) -> &Arc<EventHub> {
        &self.hub
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
    pub(crate) fn resolve(
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
    /// stream of pre-formatted SSE events. See [`Self::run_agent_raw`]
    /// for the underlying JSON-string stream the integration tests
    /// consume.
    pub fn run_agent(
        &self,
        agent: &Agent,
        prompt: String,
        history: Vec<HistoryMessage>,
    ) -> Result<
        impl Stream<Item = Result<sse::Event, std::convert::Infallible>> + Send + 'static,
        AgentRunError,
    > {
        let raw = self.run_agent_raw(agent, prompt, history)?;
        Ok(raw.map(|payload| Ok(sse::Event::default().data(payload))))
    }

    /// Same as [`Self::run_agent`] but yields the raw `data:` payload
    /// strings (one per SSE frame) instead of wrapping them in
    /// `axum::response::sse::Event`. This is the source-of-truth
    /// stream the SSE handler and the bridge integration tests both
    /// consume. The terminal `[DONE]` sentinel is always emitted.
    ///
    /// If the agent's `tools` array contains `flow:*` or `flow:<id>`
    /// entries AND its provider takes [`RunnerInput::Rest`], the
    /// runtime drives an in-process agentic loop: each turn, the
    /// runner is handed the synthesised `ToolDef`s; tool calls that
    /// match a `flow:*` name are dispatched through [`FlowEngine`]
    /// and their terminal output is fed back as a `user`-role history
    /// message before the next turn. CLI-shape runners (Claude CLI)
    /// manage their own tool loop, so flow tools are silently dropped
    /// on that path.
    pub fn run_agent_raw(
        &self,
        agent: &Agent,
        prompt: String,
        history: Vec<HistoryMessage>,
    ) -> Result<impl Stream<Item = String> + Send + 'static, AgentRunError> {
        let (provider, runner) = self.resolve(&agent.provider)?;
        let (tx, rx) = mpsc::channel::<String>(64);

        let agent = agent.clone();
        let this = self.clone();

        tokio::spawn(async move {
            let outcome = this
                .drive_chat(provider, runner, agent, prompt, history, tx.clone())
                .await;
            if let Err(msg) = outcome {
                let _ = tx
                    .send(json!({ "type": "error", "error": msg }).to_string())
                    .await;
            }
            let _ = tx.send("[DONE]".to_owned()).await;
        });

        Ok(ReceiverStream::new(rx))
    }
}
