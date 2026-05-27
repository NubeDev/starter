//! `rubix.warehouse.mart.drop` — request/response DTOs + descriptor.
//!
//! Destructive: removes a mart. The verb itself is a thin shim over
//! `WarehouseWriter::restore_mart` with an empty snapshot (the same code
//! path the undo dispatcher uses to walk back a prior `mart.create`).
//! See the data-loss caveat in
//! [docs/design/warehouse-rules/](../../../../docs/design/warehouse-rules/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartDropRequest {
    /// Fully-qualified mart name.
    pub mart_name: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseMartDropResponse {
    /// Outcome — `rubix.warehouse.mart.dropped` on the happy path
    /// or `rubix.warehouse.mart.absent` when the mart did not
    /// exist (no-op; idempotent).
    pub summary: Diagnostic,
    /// Echoed mart name.
    pub mart_name: String,
    /// Whether the mart was present at probe time.
    pub was_present: bool,
    /// Epoch milliseconds at which the DROP ran.
    pub dropped_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "warehouse.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Drop a warehouse mart. Idempotent; reports whether the table was present.",
    when_to_use: "Use when an operator says \"remove the mart for X\" and is willing to accept row loss.",
    when_not_to_use: "Do not use to roll back a recent mart.create — call rubix.undo.last instead so the snapshot record stays consistent.",
    example: concat!(
        "Input:  { \"mart_name\": \"system_disk_history\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.mart.dropped\", ",
        "\"params\": { \"mart\": \"system_disk_history\" } }, ",
        "\"mart_name\": \"system_disk_history\", \"was_present\": true, ",
        "\"dropped_at_ms\": 1764892800000 }",
    ),
    siblings: &[SiblingTool {
        id: "rubix.undo.last",
        wins_when: "the caller wants to walk back a recent mart.create with the snapshot preserved.",
    }],
};
