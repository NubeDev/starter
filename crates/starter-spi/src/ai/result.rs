//! Terminal outcomes from `AiRunner::run`.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// One captured tool invocation within a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolCallEntry {
    /// Tool name.
    pub name: String,
    /// Wall-clock duration of the tool call in milliseconds.
    pub duration_ms: u64,
    /// `"ok"` or `"error"`.
    pub status: String,
    /// Provider-supplied error message, if any.
    pub error: Option<String>,
}

/// Structured tool invocation captured from the model's output (REST
/// providers).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolUse {
    /// Provider-assigned id for this invocation.
    pub id: String,
    /// Tool name the model invoked.
    pub name: String,
    /// JSON arguments.
    pub input: JsonValue,
}

/// Aggregated result returned after a run completes.
///
/// Upstream / network / parsing failures flow through `error` rather
/// than as a typed `Err` — see `RunnerError` for the runner-layer
/// errors that *are* typed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunResult {
    /// Concatenated assistant text across all `EventKind::Text` events.
    pub text: String,
    /// Provider tag (matches `Provider::to_string`).
    pub provider: String,
    /// Model id, if reported.
    pub model: Option<String>,
    /// Upstream CLI session id for resume support (claude runner only).
    /// Not the caller's `SessionId` — this one is opaque, assigned by
    /// the binary itself, and meaningful only when fed back via
    /// `CliCfg::resume_id`.
    pub session_id: Option<String>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Provider-reported cost, if available.
    pub cost_usd: f64,
    /// Input tokens consumed.
    pub input_tokens: u32,
    /// Output tokens produced.
    pub output_tokens: u32,
    /// Number of tool calls observed.
    pub tool_calls: u32,
    /// Per-call log.
    pub tool_call_log: Vec<ToolCallEntry>,
    /// Structured tool invocations (REST providers).
    pub tool_uses: Vec<ToolUse>,
    /// Set when the run ended with a fatal upstream error.
    pub error: Option<String>,
}

/// Runner-layer errors. Upstream / network / parsing errors flow
/// through `RunResult::error`; this enum is for misuse the runner
/// detects up front.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// The runner was handed a `RunnerInput` variant it does not
    /// accept (CLI cfg passed to a REST runner, etc).
    #[error("provider `{provider}` runner expected `{expected}` input, got `{got}`")]
    WrongInputKind {
        /// Provider tag.
        provider: String,
        /// Variant the runner accepts.
        expected: &'static str,
        /// Variant the caller actually passed.
        got: &'static str,
    },
}
