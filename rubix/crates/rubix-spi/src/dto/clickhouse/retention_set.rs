//! `rubix.clickhouse.retention.set` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles). See
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md)
//! for the verb contract and the snapshot shape used by the
//! `Reversible` impl.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.clickhouse.retention.set`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseRetentionSetRequest {
    /// Fully-qualified table name (`database.table` or `table`).
    /// Becomes the snapshot resource id.
    pub table_name: String,
    /// New retention in days. `0` removes the TTL clause entirely.
    pub days: u32,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClickhouseRetentionSetResponse {
    /// Outcome — `rubix.clickhouse.retention.set` when the TTL
    /// changed, `rubix.clickhouse.retention.unchanged` when the
    /// requested value already matches what the table carries
    /// (no-op; no Change is recorded for undo).
    pub summary: Diagnostic,
    /// Echoed table name.
    pub table_name: String,
    /// Prior retention in days; `None` when the table had no TTL
    /// clause before this call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_days: Option<u32>,
    /// New retention in days (echoes the request).
    pub days: u32,
    /// Whether the call left the TTL unchanged.
    pub was_unchanged: bool,
    /// Epoch milliseconds (UTC) at which the ALTER ran (or the
    /// no-op was observed).
    pub set_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "clickhouse.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Set the TTL/retention (in days) on a ClickHouse history or mart table.",
    when_to_use: concat!(
        "Use when an operator says \"keep X for N days\", \"prune ",
        "older than\", or when a flow needs to align retention with ",
        "a compliance window."
    ),
    when_not_to_use: concat!(
        "Do not use to create or alter the table shape (those are ",
        "rubix.clickhouse.mart.create / rubix.clickhouse.rule.write). ",
        "Setting `days = 0` removes the TTL clause but does NOT drop ",
        "the table — undo restores the prior TTL."
    ),
    example: concat!(
        "Input:  { \"table_name\": \"system_disk_history\", \"days\": 30 }\n",
        "Output: { \"summary\": { \"code\": \"rubix.clickhouse.retention.set\", ",
        "\"params\": { \"table\": \"system_disk_history\", \"days\": 30 } }, ",
        "\"table_name\": \"system_disk_history\", \"prior_days\": 90, ",
        "\"days\": 30, \"was_unchanged\": false, \"set_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.clickhouse.mart.create",
            wins_when: "the table does not exist yet — create it first, then set retention.",
        },
        SiblingTool {
            id: "rubix.clickhouse.rule.write",
            wins_when: "the change is structural (columns, ORDER BY, engine), not TTL.",
        },
    ],
};
