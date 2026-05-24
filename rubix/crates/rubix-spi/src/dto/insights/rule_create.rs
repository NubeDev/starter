//! `rubix.insights.rule.create` — request/response DTOs + descriptor.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleCreateRequest {
    /// Stable id (becomes the resource key).
    pub rule_id: String,
    /// Raw YAML body. Validation is parse-only at this layer (the
    /// insights engine re-parses on evaluation).
    pub body_yaml: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleCreateResponse {
    /// Outcome — `rubix.insights.rule.created` on the happy path
    /// or `rubix.insights.rule.replaced` when the id already
    /// existed (idempotent overwrite).
    pub summary: Diagnostic,
    /// Echoed rule id.
    pub rule_id: String,
    /// Epoch milliseconds at which the rule was written.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "insights.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Create or replace an insights rule from a YAML body.",
    when_to_use: "Use when an operator says \"add an insights rule for X\" or when a flow needs to provision a new alert path.",
    when_not_to_use: "Do not use to merely toggle an existing rule (use rubix.insights.rule.enable / disable).",
    example: concat!(
        "Input:  { \"rule_id\": \"disk-high\", \"body_yaml\": \"when: disk.used_pct > 90\\n\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.insights.rule.created\", ",
        "\"params\": { \"rule\": \"disk-high\" } }, ",
        "\"rule_id\": \"disk-high\", \"created_at_ms\": 1764892800000 }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.insights.rule.enable",
        wins_when: "the rule already exists and the caller only wants to flip the active flag.",
    }],
};
