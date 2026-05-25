//! `rubix.insights.rule.{enable, disable}` — shared toggle DTOs +
//! descriptor.
//!
//! The two verbs share a request/response shape (only the `enabled`
//! field of the response differs); the agent registers two distinct
//! `Tool` entries so the diagnostic code reflects the intent.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for both enable and disable.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleToggleRequest {
    /// Stable id of the rule to toggle.
    pub rule_id: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleToggleResponse {
    /// Outcome — `rubix.insights.rule.enabled` /
    /// `rubix.insights.rule.disabled` on the happy path or
    /// `rubix.insights.rule.not_found` when the id is unknown.
    pub summary: Diagnostic,
    /// Echoed rule id.
    pub rule_id: String,
    /// New active flag (the post-toggle state).
    pub enabled: bool,
    /// Epoch milliseconds at which the flip ran.
    pub toggled_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold for both
/// `enable` and `disable`.
pub const REQUIRED_PERMISSION: &str = "insights.write";

/// Descriptor for `rubix.insights.rule.enable`.
pub static DESCRIPTOR_ENABLE: ToolDescriptor = ToolDescriptor {
    purpose: "Enable an existing insights rule. Idempotent.",
    when_to_use: "Use when an operator says \"turn on the disk-high alert\" or a flow needs to reactivate a previously-paused rule.",
    when_not_to_use: "Do not use to create a rule (use rubix.insights.rule.create).",
    example: concat!(
        "Input:  { \"rule_id\": \"disk-high\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.insights.rule.enabled\", ",
        "\"params\": { \"rule\": \"disk-high\" } }, ",
        "\"rule_id\": \"disk-high\", \"enabled\": true, ",
        "\"toggled_at_ms\": 1764892800000 }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.insights.rule.disable",
        wins_when: "the operator wants to PAUSE the rule.",
    }],
};

/// Descriptor for `rubix.insights.rule.disable`.
pub static DESCRIPTOR_DISABLE: ToolDescriptor = ToolDescriptor {
    purpose: "Disable an existing insights rule. Idempotent.",
    when_to_use: "Use when an operator says \"pause the noisy alert\" without removing the rule body.",
    when_not_to_use: "Do not use to delete a rule outright; deletion is not yet exposed as a tool — disable + ignore is the documented workaround.",
    example: concat!(
        "Input:  { \"rule_id\": \"disk-high\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.insights.rule.disabled\", ",
        "\"params\": { \"rule\": \"disk-high\" } }, ",
        "\"rule_id\": \"disk-high\", \"enabled\": false, ",
        "\"toggled_at_ms\": 1764892800000 }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.insights.rule.enable",
        wins_when: "the operator wants to RESUME the rule.",
    }],
};
