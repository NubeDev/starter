//! Tool-call ⇄ history serialisation helpers.
//!
//! Kept separate from the loop body so the (small but tedious) text-
//! shape decisions live in one place. Today the loop carries the
//! conversation forward in the [`starter_spi::ai::input::RestCfg`]
//! `history` field with one `assistant` turn capturing the model's
//! tool-use intent and a subsequent `user` turn carrying the rendered
//! tool results. The shape is deliberately simple — the long-term
//! plan replaces it with provider-native tool-result message parts.

use starter_spi::ai::{HistoryMessage, ToolUse};

/// Render the model's tool-use intent as a single assistant turn.
pub fn assistant_tool_use_message(prior_text: &str, calls: &[ToolUse]) -> HistoryMessage {
    let mut body = prior_text.trim().to_owned();
    for call in calls {
        body.push_str(&format!(
            "\n[tool_use name=\"{}\" input={}]",
            call.name, call.input
        ));
    }
    HistoryMessage {
        role: "assistant".into(),
        content: body,
    }
}

/// Render the collected tool results as a single follow-up user turn.
pub fn user_tool_results_message(results: &[(String, serde_json::Value)]) -> HistoryMessage {
    let mut body = String::new();
    for (name, value) in results {
        body.push_str(&format!("[tool_result name=\"{name}\" output={value}]\n"));
    }
    HistoryMessage {
        role: "user".into(),
        content: body,
    }
}
