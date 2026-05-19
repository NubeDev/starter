//! `RunnerInput` — typed input handed to `AiRunner::run`.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// One message in a multi-turn conversation (REST providers).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    /// `"system"`, `"user"`, or `"assistant"`.
    pub role: String,
    /// Message body.
    pub content: String,
}

/// Pluggable headless-permission mode for CLI runners. Mirrors
/// `claude-wrapper::PermissionMode` for the claude path; other runners
/// may interpret a subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    /// Wrapper's interactive default. Fine when a human is at the TTY.
    Default,
    /// Auto-approve filesystem edits but still prompt for shell.
    AcceptEdits,
    /// Plan-only mode; do not execute tools.
    Plan,
    /// Bypass every permission check. Required for headless runs.
    Bypass,
}

/// A tool the model may invoke. Mirrors the Anthropic / OpenAI schema
/// shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name as the model will see it.
    pub name: String,
    /// Human-readable description shown to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema for the tool's input object.
    pub input_schema: JsonValue,
}

/// Constraint on tool selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Model decides whether to call a tool.
    Auto,
    /// Model must call some tool.
    Any,
    /// Model must call the named tool.
    Tool {
        /// Name of the required tool.
        name: String,
    },
    /// Model must not call any tool.
    None,
}

/// Configuration for a CLI-transport run (claude-wrapper, codex CLI).
#[derive(Debug, Clone, Default)]
pub struct CliCfg {
    /// The user prompt.
    pub prompt: String,
    /// Optional system prompt / context.
    pub system_prompt: Option<String>,
    /// Model override, e.g. `"claude-opus-4-5"`.
    pub model: Option<String>,
    /// Resume a previous CLI session by its session ID.
    pub resume_id: Option<String>,
    /// MCP server URL.
    pub mcp_url: Option<String>,
    /// Bearer token for MCP server auth.
    pub mcp_token: Option<String>,
    /// Path to a pre-built MCP config JSON file. When set, this is
    /// passed directly via `--mcp-config` and `mcp_url` / `mcp_token`
    /// are ignored.
    pub mcp_config_path: Option<String>,
    /// Tool filter pattern, e.g. `"mcp__acme__*"`.
    pub allowed_tools: Option<String>,
    /// Thinking budget: `"low"`, `"medium"`, `"high"`, or a token count.
    pub thinking_budget: Option<String>,
    /// Working directory for the spawned subprocess.
    pub work_dir: Option<String>,
    /// Provider-agnostic permission mode for CLI wrappers that gate
    /// filesystem / shell tool calls behind an approval prompt. `None`
    /// keeps the wrapper's interactive default — fatal for headless.
    pub permission_mode: Option<PermissionMode>,
    /// Built-in tool whitelist (Bash, Read, Edit, …). Comma-separated,
    /// forwarded to the claude binary's `--tools` flag.
    pub tools: Option<String>,
}

/// Configuration for a REST-transport run (Anthropic, OpenAI cloud
/// APIs).
#[derive(Debug, Clone, Default)]
pub struct RestCfg {
    /// The user prompt.
    pub prompt: String,
    /// Optional system prompt / context.
    pub system_prompt: Option<String>,
    /// Model override, e.g. `"gpt-4o"`.
    pub model: Option<String>,
    /// API key. Falls back to the standard env var when absent.
    pub api_key: Option<String>,
    /// Base URL override (proxies, local servers).
    pub base_url: Option<String>,
    /// Pre-loaded conversation history.
    pub history: Vec<HistoryMessage>,
    /// Maximum tokens to generate.
    pub max_tokens: Option<u32>,
    /// Extra HTTP headers forwarded verbatim.
    pub extra_headers: HashMap<String, String>,
    /// Tools exposed to the model for structured output / function
    /// calling.
    pub tools: Vec<ToolDef>,
    /// How the model is allowed / required to pick a tool.
    pub tool_choice: Option<ToolChoice>,
    /// Thinking budget: `"low"`, `"medium"`, `"high"`, or a token count.
    pub thinking_budget: Option<String>,
}

/// Typed input for a single run. A runner handed the wrong variant
/// returns `RunnerError::WrongInputKind` rather than silently dropping
/// fields the caller populated.
#[derive(Debug, Clone)]
pub enum RunnerInput {
    /// CLI-shaped run (subprocess, binary-managed auth).
    Cli(CliCfg),
    /// REST-shaped run (HTTP API, key-managed auth).
    Rest(RestCfg),
}

impl RunnerInput {
    /// Short tag used in error messages and tracing.
    pub fn kind_tag(&self) -> &'static str {
        match self {
            RunnerInput::Cli(_) => "cli",
            RunnerInput::Rest(_) => "rest",
        }
    }
}
