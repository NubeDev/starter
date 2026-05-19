//! `Provider` — which AI vendor an `AiRunner` represents.

use serde::{Deserialize, Serialize};

/// Identifier for the underlying AI vendor / transport. Lifted from
/// `codeless-workspace/ai-runner` per SCOPE q7.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    /// Claude Code CLI — auth managed by the `claude` binary itself.
    Claude,
    /// OpenAI Codex CLI — reads `OPENAI_API_KEY` from environment.
    Codex,
    /// GitHub Copilot CLI — auth managed by the `copilot` binary itself.
    Copilot,
    /// Anthropic cloud REST API — key via `RestCfg::api_key` or
    /// `ANTHROPIC_API_KEY`.
    Anthropic,
    /// OpenAI cloud REST API — key via `RestCfg::api_key` or
    /// `OPENAI_API_KEY`.
    OpenAi,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Provider::Claude => "claude",
            Provider::Codex => "codex",
            Provider::Copilot => "copilot",
            Provider::Anthropic => "anthropic",
            Provider::OpenAi => "openai",
        };
        f.write_str(s)
    }
}
