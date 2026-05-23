//! `rubix.alert.send` — request/response DTOs and tool descriptor.
//!
//! Emits a single operator alert. v0 sends to the local tracing
//! sink only; real downstream channels (email / webhook / paging)
//! arrive with the alert-sink wiring described in
//! [docs/design/audit/](../../../../docs/design/audit/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Alert severity. Mirrors the tracing log levels callers are
/// already used to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AlertSeverity {
    /// Operator-visible note. Never pages.
    Info,
    /// Worth attention soon; does not page.
    Warn,
    /// Operator should act; the downstream sink decides whether to page.
    Error,
}

/// Caller input for `rubix.alert.send`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertSendRequest {
    /// Severity to attach to the emitted alert.
    pub severity: AlertSeverity,
    /// Short, operator-readable message body. Truncated to 1024
    /// chars by the dispatch layer.
    pub message: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlertSendResponse {
    /// Outcome keyed for the transport layer to render.
    pub summary: Diagnostic,
    /// Severity actually emitted (matches the request).
    pub severity: AlertSeverity,
    /// Length of the message body that reached the sink, in chars.
    pub delivered_chars: u32,
    /// Epoch milliseconds (UTC) at which the alert was emitted.
    pub probed_at_ms: i64,
}

/// Hard cap on `message` length the dispatch layer applies before
/// forwarding to the sink. Anything longer is truncated; the response
/// reports the post-truncation length.
pub const MESSAGE_MAX_CHARS: usize = 1024;

/// `starter-authz` permission string the caller must hold to invoke
/// this tool. Write verbs ride on a separate permission from read
/// verbs so an operator can be granted observation without alerting.
/// See [docs/design/auth/](../../../../docs/design/auth/README.md).
pub const REQUIRED_PERMISSION: &str = "system.alert";

/// Five-field descriptor for `rubix.alert.send`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Emit a single operator alert from the rubix-agent host via the configured alert sink.",
    when_to_use: concat!(
        "Use when a skill has concluded that an operator must be notified ",
        "(disk full, repeated flow errors, auth lockouts). One call per ",
        "operator-visible event; downstream sink handles deduplication."
    ),
    when_not_to_use: concat!(
        "Do not use for routine progress logging (use tracing). Do not use ",
        "as a structured-event channel (use rubix.flow.* or audit logs). ",
        "Do not loop: the alert sink is the place to dedupe, not the caller."
    ),
    example: concat!(
        "Input:  { \"severity\": \"warn\", \"message\": \"Disk 89% full on /\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.alert.send.ok\", ",
        "\"params\": { \"severity\": \"warn\", \"at\": <epoch_ms> } }, ",
        "\"severity\": \"warn\", \"delivered_chars\": 21, ",
        "\"probed_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.system.flow_errors",
            wins_when: "the caller wants to count past errors rather than emit a new alert.",
        },
        SiblingTool {
            id: "rubix.system.db",
            wins_when: "the caller wants engine reachability rather than to notify an operator.",
        },
    ],
};
