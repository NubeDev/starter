//! `rubix.warehouse.mart.list` — request/response DTOs + descriptor.
//!
//! Read-only sibling of `rubix.warehouse.mart.create`. Surfaces
//! every mart the backing [`WarehouseWriter`](
//! crate::dto::clickhouse::mart_create::WarehouseMartCreateRequest)
//! holds.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input. Empty for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartListRequest {}

/// One mart as returned by `rubix.warehouse.mart.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseMartSummary {
    /// Fully-qualified mart name.
    pub mart_name: String,
    /// Current `SHOW CREATE TABLE` body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ddl: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartListResponse {
    /// Outcome (`rubix.warehouse.mart.listed`).
    pub summary: Diagnostic,
    /// Total row count.
    pub count: usize,
    /// Rows sorted by `mart_name` ascending.
    pub marts: Vec<ClickhouseMartSummary>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "clickhouse.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List ClickHouse marts (history/aggregate tables) currently registered with the agent.",
    when_to_use: "Use to render the clickhouse-ruler admin UI's marts table or to confirm a mart name before a write.",
    when_not_to_use: "Do not use to enumerate derived-state rules (rubix.warehouse.rule.list) or raw tables (rubix.warehouse.tables.list).",
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.mart.listed\", ",
        "\"params\": { \"count\": 1 } }, \"count\": 1, ",
        "\"marts\": [ { \"mart_name\": \"system_disk_history\", ",
        "\"ddl\": \"CREATE TABLE ...\" } ] }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.warehouse.mart.create",
        wins_when: "the caller wants to PROVISION a mart, not enumerate.",
    }],
};
