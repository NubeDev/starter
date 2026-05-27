//! `rubix.warehouse.rule.write` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles). See
//! [docs/design/warehouse-rules/](../../../../docs/design/warehouse-rules/README.md)
//! for the verb contract and the snapshot shape used by the
//! `Reversible` impl.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.warehouse.rule.write`.
///
/// A "rule" is a derived-state warehouse object — typically a
/// MATERIALIZED VIEW or continuous aggregate that aggregates raw
/// samples into an L2/L3 rollup. The verb
/// writes the supplied DDL verbatim; validation is parse-only
/// (the verb refuses non-`CREATE`/`ALTER` statements).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseRuleWriteRequest {
    /// Fully-qualified target — `database.table` or just `table`
    /// when the agent's default database applies. Used both as the
    /// stable resource id for the undo snapshot and to drive the
    /// `SHOW CREATE TABLE` snapshot probe.
    pub rule_name: String,
    /// Raw DDL to execute. Must begin with `CREATE ` or
    /// `ALTER ` (case-insensitive). `DROP` is refused — destructive
    /// removals go through the inverse path of a prior write.
    pub ddl: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseRuleWriteResponse {
    /// Outcome (`rubix.warehouse.rule.written`).
    pub summary: Diagnostic,
    /// Echoed rule name (resource id of the snapshot row).
    pub rule_name: String,
    /// Prior `SHOW CREATE TABLE` body, or `None` when the rule did
    /// not exist before this call. Surfaced in the response so
    /// integration tests can assert the snapshot without round-
    /// tripping through `undo_snapshots`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_ddl: Option<String>,
    /// Epoch milliseconds (UTC) at which the DDL ran.
    pub written_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "warehouse.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Write or alter a warehouse derived-state rule (MATERIALIZED VIEW, continuous aggregate).",
    when_to_use: concat!(
        "Use when an operator says \"add a rollup\", \"change the ",
        "aggregation for X\", or when a flow needs to deploy a new ",
        "derived-state object alongside an existing history table."
    ),
    when_not_to_use: concat!(
        "Do not use to create a brand-new mart table (call ",
        "rubix.warehouse.mart.create). Do not use to change ",
        "retention/TTL (call rubix.warehouse.retention.set). Never ",
        "use to DROP an object — undo the prior write instead."
    ),
    example: concat!(
        "Input:  { \"rule_name\": \"system_disk_rollup_1h\", ",
        "\"ddl\": \"CREATE MATERIALIZED VIEW ...\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.rule.written\", ",
        "\"params\": { \"rule\": \"system_disk_rollup_1h\" } }, ",
        "\"rule_name\": \"system_disk_rollup_1h\", ",
        "\"prior_ddl\": null, \"written_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.warehouse.mart.create",
            wins_when: "the target is a fresh history/mart table, not a view over an existing one.",
        },
        SiblingTool {
            id: "rubix.warehouse.retention.set",
            wins_when: "the caller only wants to change how long data is kept, not the shape.",
        },
    ],
};
