//! Streamed events from an in-flight AI run.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::session::SessionId;

/// A normalised streaming event emitted by any provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Caller-supplied identifier grouping all events for one run.
    pub session_id: SessionId,
    /// Provider that produced this event.
    pub provider: String,
    /// Typed payload.
    pub kind: EventKind,
}

/// The typed payload of an [`Event`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    /// Backend process / HTTP stream established.
    Connected {
        /// Model id reported by the provider, if any.
        model: Option<String>,
    },
    /// A chunk of generated text.
    Text {
        /// New text appended to the conversation.
        content: String,
    },
    /// The model invoked a tool. `id` and `input` are present for REST
    /// providers (Anthropic, OpenAI) that supply a structured tool
    /// block, and absent for CLI providers (Claude wrapper, Codex)
    /// which only surface the name.
    ToolUse {
        /// Provider-assigned id for this invocation.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// Tool name the model wants to invoke.
        name: String,
        /// JSON arguments. Absent for CLI providers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        input: Option<JsonValue>,
    },
    /// Run finished successfully.
    Done {
        /// Wall-clock duration in milliseconds.
        duration_ms: u64,
        /// Provider-reported cost, if available.
        cost_usd: f64,
        /// Input tokens consumed.
        input_tokens: u32,
        /// Output tokens produced.
        output_tokens: u32,
    },
    /// Something went wrong.
    Error {
        /// Human-readable error message.
        message: String,
    },
}
