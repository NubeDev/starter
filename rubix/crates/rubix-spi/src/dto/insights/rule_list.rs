//! `rubix.insights.rule.list` — request/response DTOs + descriptor.
//!
//! Read-only sibling of `rubix.insights.rule.create`. Surfaces every
//! insights rule the backing store currently holds, sorted by
//! `rule_id` for stable rendering. The hook
//! `useInsightsRulesList` in
//! `rubix/packages/rubix-client-react/src/hooks/insights.ts` is the
//! primary consumer.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input. Empty for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleListRequest {}

/// One rule as returned by `rubix.insights.rule.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleSummary {
    /// Stable id.
    pub rule_id: String,
    /// Human-facing name (defaults to `rule_id` if none was set).
    pub name: String,
    /// Whether the rule is currently active.
    pub enabled: bool,
    /// Raw YAML body of the rule. `None` when the store does not
    /// retain it (rare — the in-memory backing always carries it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_yaml: Option<String>,
    /// Epoch milliseconds of the most recent write.
    pub updated_at_ms: i64,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InsightsRuleListResponse {
    /// Outcome (`rubix.insights.rule.listed`).
    pub summary: Diagnostic,
    /// Total row count.
    pub count: usize,
    /// Rows sorted by `rule_id` ascending.
    pub rules: Vec<InsightsRuleSummary>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "insights.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List insights rules currently registered with the agent, with enable state and YAML body.",
    when_to_use: "Use to render the insights admin UI's rules table or to confirm a rule_id before enable/disable/create.",
    when_not_to_use: "Do not use to evaluate or fire an insights rule — that path is internal to the insights engine.",
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.insights.rule.listed\", ",
        "\"params\": { \"count\": 1 } }, \"count\": 1, ",
        "\"rules\": [ { \"rule_id\": \"disk-high\", \"name\": \"disk-high\", ",
        "\"enabled\": true, \"updated_at_ms\": 1764892800000 } ] }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.insights.rule.create",
        wins_when: "the caller wants to ADD a rule, not enumerate.",
    }],
};
