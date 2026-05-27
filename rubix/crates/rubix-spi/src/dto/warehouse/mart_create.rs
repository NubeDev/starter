//! `rubix.warehouse.mart.create` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles). See
//! [docs/design/warehouse-rules/](../../../../docs/design/warehouse-rules/README.md)
//! for the verb contract and the snapshot shape used by the
//! `Reversible` impl, including the data-loss caveat on undo when
//! the prior snapshot is empty (the table did not exist before).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.warehouse.mart.create`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartCreateRequest {
    /// Fully-qualified mart name (`database.table` or `table`).
    /// Becomes the snapshot resource id.
    pub mart_name: String,
    /// `CREATE TABLE` DDL to execute. Must begin with `CREATE TABLE`
    /// (case-insensitive); other shapes are refused.
    pub ddl: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartCreateResponse {
    /// Outcome — `rubix.warehouse.mart.created` on the happy path
    /// or `rubix.warehouse.mart.already_exists` when the mart was
    /// already present (no-op; the verb is idempotent).
    pub summary: Diagnostic,
    /// Echoed mart name.
    pub mart_name: String,
    /// Prior `SHOW CREATE TABLE` body, or `None` when the mart did
    /// not exist before this call. When `None` the inverse op is
    /// `DROP TABLE IF EXISTS` — undo recovers the schema but NOT
    /// the rows ingested between the create and the undo. This
    /// caveat is documented in the design doc and surfaced in the
    /// diagnostic params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_ddl: Option<String>,
    /// Whether the verb was a no-op because the mart already existed.
    pub was_already_present: bool,
    /// Epoch milliseconds (UTC) at which the DDL ran.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "warehouse.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Create a new warehouse mart (history/aggregate table) from a CREATE TABLE DDL.",
    when_to_use: concat!(
        "Use when an operator says \"add a new history table for X\" ",
        "or when a flow needs to provision the storage target for a ",
        "later rule.write."
    ),
    when_not_to_use: concat!(
        "Do not use to alter an existing table's columns (use ",
        "rubix.warehouse.rule.write with an ALTER statement). Do not ",
        "use to set TTL (rubix.warehouse.retention.set). Note: undo ",
        "of a create against a brand-new mart issues DROP TABLE — any ",
        "rows ingested in between are lost."
    ),
    example: concat!(
        "Input:  { \"mart_name\": \"system_disk_history\", ",
        "\"ddl\": \"CREATE TABLE system_disk_history (ts TIMESTAMPTZ NOT NULL, ...)\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.mart.created\", ",
        "\"params\": { \"mart\": \"system_disk_history\" } }, ",
        "\"mart_name\": \"system_disk_history\", \"prior_ddl\": null, ",
        "\"was_already_present\": false, \"created_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.warehouse.rule.write",
            wins_when:
                "the target is a derived-state view over an existing mart, not a fresh table.",
        },
        SiblingTool {
            id: "rubix.warehouse.retention.set",
            wins_when: "the caller wants to set or change the TTL on an existing mart.",
        },
    ],
};
