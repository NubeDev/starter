//! The unified streaming event. Both inference streaming (token deltas) and
//! agent runs (tool calls, file edits, progress, final output) get normalised
//! into one enum so a caller can drive either with the same match arms. Each
//! layer maps its richer native events down into these; provider-specific detail
//! that doesn't fit is preserved in [`Event::Raw`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// A chunk of assistant text. Emitted by both layers.
    TextDelta { text: String },

    /// The model/agent invoked a tool or function. `input` is the raw JSON args.
    ToolCall {
        name: String,
        input: serde_json::Value,
    },

    /// Agent-only: a human-readable progress note (e.g. "editing authz.rs").
    Progress { message: String },

    /// Terminal event. Carries the full final text and, when available, a usage
    /// summary. After this no more events arrive on the stream.
    Done {
        text: String,
        usage: Option<Usage>,
    },

    /// An escape hatch: a provider-native event we chose not to flatten, kept so
    /// power users aren't boxed in by the lowest-common-denominator surface.
    Raw(serde_json::Value),
}

/// Token accounting, when the provider reports it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}
