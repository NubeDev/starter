//! `rubix.clickhouse.rule.list` — request/response DTOs + descriptor.
//!
//! Read-only sibling of `rubix.clickhouse.rule.write`. The backing
//! `ChWriter::list_rules` surfaces every rule the in-memory or
//! CH-backed store currently holds; this DTO mirrors the loose
//! shape the rubix-client-react `useClickhouseRulesList` hook
//! expects. See
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input. Empty for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseRuleListRequest {}

/// One rule as returned by `rubix.clickhouse.rule.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseRuleSummary {
    /// Fully-qualified rule name.
    pub rule_name: String,
    /// Current `SHOW CREATE TABLE` body for this rule. `None`
    /// when the backing store cannot reproduce the DDL (rare —
    /// the in-memory impl always carries it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ddl: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseRuleListResponse {
    /// Outcome (`rubix.clickhouse.rule.listed`).
    pub summary: Diagnostic,
    /// Total row count.
    pub count: usize,
    /// Rows sorted by `rule_name` ascending for stable rendering.
    pub rules: Vec<ClickhouseRuleSummary>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "clickhouse.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List ClickHouse derived-state rules currently registered with the agent.",
    when_to_use: concat!(
        "Use to render the clickhouse-ruler admin UI's rules table, ",
        "or to let an agent confirm a rule name before a follow-up ",
        "rule.write / undo.",
    ),
    when_not_to_use: concat!(
        "Do not use to enumerate marts (use rubix.clickhouse.mart.list) ",
        "or raw tables (use rubix.clickhouse.tables.list).",
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.clickhouse.rule.listed\", ",
        "\"params\": { \"count\": 1 } }, \"count\": 1, ",
        "\"rules\": [ { \"rule_name\": \"system_disk_rollup_1h\", ",
        "\"ddl\": \"CREATE MATERIALIZED VIEW ...\" } ] }",
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.clickhouse.rule.write",
            wins_when: "the caller wants to CREATE or ALTER a rule, not enumerate.",
        },
        SiblingTool {
            id: "rubix.clickhouse.mart.list",
            wins_when: "the caller wants storage tables, not derived-state views.",
        },
    ],
};
