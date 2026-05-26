//! `rubix.warehouse.tables.list` — request/response DTOs + descriptor.
//!
//! Surfaces every table the backing store knows about — for the
//! in-memory impl that is the union of marts and TTL-tracked
//! retention targets. The CH-backed impl will return rows from
//! `system.tables`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input. Empty for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct WarehouseTablesListRequest {}

/// One table as returned by `rubix.warehouse.tables.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseTableSummary {
    /// Fully-qualified table name.
    pub table_name: String,
    /// Engine (e.g. `MergeTree`, `ReplicatedMergeTree`).
    pub engine: String,
    /// Current TTL in days; `None` when none is set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_days: Option<u32>,
    /// Approximate row count; `None` when unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_count: Option<u64>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseTablesListResponse {
    /// Outcome (`rubix.warehouse.tables.listed`).
    pub summary: Diagnostic,
    /// Total row count.
    pub count: usize,
    /// Rows sorted by `table_name` ascending.
    pub tables: Vec<ClickhouseTableSummary>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "clickhouse.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List every ClickHouse table the agent knows about, with engine and TTL.",
    when_to_use: "Use for the warehouse admin overview, or to pick a `table_name` before rubix.warehouse.retention.set.",
    when_not_to_use: "Do not use to enumerate derived-state rules (rubix.warehouse.rule.list).",
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.tables.listed\", ",
        "\"params\": { \"count\": 1 } }, \"count\": 1, ",
        "\"tables\": [ { \"table_name\": \"system_disk_history\", ",
        "\"engine\": \"MergeTree\", \"retention_days\": 90 } ] }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.warehouse.retention.set",
        wins_when: "the caller wants to MUTATE a TTL after picking the table from this list.",
    }],
};
