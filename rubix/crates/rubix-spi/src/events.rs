//! Typed agent SSE event taxonomy.
//!
//! These shapes are the wire contract for rubix's stream surface,
//! shared across SSE, gRPC stream, and MCP `notifications/progress`.
//! See [docs/design/events/](../../docs/design/events/README.md) for
//! the taxonomy and the planned migration to `starter-flow`.
//!
//! Strings in events are MessageKey, not raw text — the transport
//! resolves to the caller's locale via starter-i18n. A tool emitting
//! `"Disk full"` instead of `MessageKey::new("rubix.system.disk_full")`
//! is a bug.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Every typed event rubix emits over SSE / gRPC stream / MCP
/// `notifications/progress`. Shared across transports.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentEvent {
    /// New agent turn started. Carries the active skill so observers
    /// can correlate skill activity with downstream events.
    TurnStart { turn_id: String, skill: Option<String> },
    /// Partial LLM token stream. Transport maps to
    /// `notifications/progress` for MCP clients.
    Thinking { turn_id: String, delta: String },
    /// Tool dispatch starting.
    ToolStart { turn_id: String, tool: String, args: serde_json::Value },
    /// Tool dispatch finished successfully.
    ToolComplete { turn_id: String, tool: String, duration_ms: u64 },
    /// Tool dispatch failed (including `SkillForbidden` per R7).
    ToolError { turn_id: String, tool: String, message_key: String },
    /// Long-running tool progress (≥1 / 5s while running per R13).
    Progress { turn_id: String, percent: Option<u8>, message_key: String },
    /// Flow engine advanced to the next node.
    FlowStep { node_id: String },
    /// Slot was written (path / before / after — exact shape TBD when
    /// starter-flow event taxonomy lands).
    SlotWrite { path: String },
    /// SkillSelector picked a skill for the current run.
    SkillMatch { skill: String, score: f32 },
}
