//! AI runner selection at boot.
//!
//! Returns the single [`Arc<dyn AiRunner>`] the host hands to every
//! `ai-agent` node. The selection is driven by
//! [`AgentConfig::ai_provider`](super::config::AgentConfig::ai_provider)
//! and an optional `RUBIX_AI_FIXTURE` env-var escape hatch that
//! swaps in a JSON-script replay runner for integration tests —
//! see [docs/design/ai-providers/](../../../docs/design/ai-providers/README.md).
//!
//! For v0 only the `claude-cli` provider is wired live. The
//! `anthropic` REST branch returns [`AiError::Unimplemented`] —
//! the deferred work is captured in
//! [crates/starter-ai-agent/LONG-TERM.md](../../../../crates/starter-ai-agent/LONG-TERM.md).

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::mpsc;

use starter_ai::runners::claude::ClaudeRunner;
use starter_spi::ai::{
    AiRunner, Cancel, Event, Provider, RunResult, RunnerError, RunnerInput, SessionId, ToolUse,
};

use super::config::AgentConfig;

/// Error returned by [`build_runner`].
#[derive(Debug)]
#[non_exhaustive]
pub enum AiError {
    /// The configured provider is recognised but not yet wired into
    /// `rubix-agent`. The message points at the LONG-TERM doc.
    Unimplemented(String),
    /// The configured provider string is not a known
    /// [`starter_spi::ai::Provider`] variant.
    Unknown(String),
    /// Fixture-mode fault (path missing or JSON malformed).
    Fixture(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unimplemented(p) => write!(
                f,
                "ai_provider `{p}`: not implemented yet — see crates/starter-ai-agent/LONG-TERM.md"
            ),
            Self::Unknown(p) => write!(
                f,
                "ai_provider `{p}`: unknown — expected one of claude-cli|anthropic"
            ),
            Self::Fixture(m) => write!(f, "RUBIX_AI_FIXTURE: {m}"),
        }
    }
}

impl std::error::Error for AiError {}

/// Default provider string used when [`AgentConfig::ai_provider`] is
/// unset — matches the commented sample in `rubix/dev/agent.toml`.
pub const DEFAULT_PROVIDER: &str = "claude-cli";

/// Build the runner per config.
///
/// - `RUBIX_AI_FIXTURE` set → fixture-replay runner (CI / integration tests).
/// - `ai_provider = "claude-cli"` (default) → [`ClaudeRunner`].
/// - `ai_provider = "anthropic"` → [`AiError::Unimplemented`].
pub fn build_runner(cfg: &AgentConfig) -> Result<Arc<dyn AiRunner>, AiError> {
    if let Ok(path) = std::env::var("RUBIX_AI_FIXTURE") {
        return FixtureRunner::load(&path).map(|r| Arc::new(r) as Arc<dyn AiRunner>);
    }
    let provider = cfg.ai_provider.as_deref().unwrap_or(DEFAULT_PROVIDER);
    match provider {
        "claude-cli" | "claude" => Ok(Arc::new(ClaudeRunner) as Arc<dyn AiRunner>),
        "anthropic" => Err(AiError::Unimplemented("anthropic".into())),
        other => Err(AiError::Unknown(other.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Fixture-replay runner — recorded-LLM transcript for integration tests.
// ---------------------------------------------------------------------------

/// One scripted turn. `tool_uses` empty = terminal (loop returns
/// `text`); non-empty = AgentLoop dispatches the tools then re-calls.
#[derive(Debug, Clone, serde::Deserialize)]
struct ScriptTurn {
    #[serde(default)]
    text: String,
    #[serde(default)]
    tool_uses: Vec<ToolUseFixture>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ToolUseFixture {
    id: String,
    name: String,
    #[serde(default)]
    input: serde_json::Value,
}

/// JSON-script runner. The transcript is `[{text, tool_uses: [...]}, ...]`;
/// each `AiRunner::run` call pops the next turn.
pub(crate) struct FixtureRunner {
    script: std::sync::Mutex<std::collections::VecDeque<ScriptTurn>>,
}

impl FixtureRunner {
    fn load(path: &str) -> Result<Self, AiError> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| AiError::Fixture(format!("read `{path}`: {e}")))?;
        let script: Vec<ScriptTurn> = serde_json::from_str(&raw)
            .map_err(|e| AiError::Fixture(format!("parse `{path}`: {e}")))?;
        Ok(Self {
            script: std::sync::Mutex::new(script.into()),
        })
    }
}

#[async_trait]
impl AiRunner for FixtureRunner {
    fn provider(&self) -> &Provider {
        &Provider::Anthropic
    }
    async fn ready(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _input: RunnerInput,
        _session: SessionId,
        _on_event: mpsc::Sender<Event>,
        _cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError> {
        let turn = self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(ScriptTurn {
                text: "(no more scripted turns)".into(),
                tool_uses: Vec::new(),
            });
        Ok(RunResult {
            text: turn.text,
            tool_uses: turn
                .tool_uses
                .into_iter()
                .map(|t| ToolUse {
                    id: t.id,
                    name: t.name,
                    input: t.input,
                })
                .collect(),
            provider: Provider::Anthropic.to_string(),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_provider_yields_claude_runner() {
        let cfg = AgentConfig::default();
        // `RUBIX_AI_FIXTURE` could be set by the caller's env — clear it
        // for this assertion so we exercise the default branch.
        std::env::remove_var("RUBIX_AI_FIXTURE");
        let r = build_runner(&cfg).expect("default builds");
        assert_eq!(r.provider().to_string(), "claude");
    }

    #[test]
    fn anthropic_is_unimplemented() {
        std::env::remove_var("RUBIX_AI_FIXTURE");
        let mut cfg = AgentConfig::default();
        cfg.ai_provider = Some("anthropic".into());
        assert!(matches!(build_runner(&cfg), Err(AiError::Unimplemented(_))));
    }
}
