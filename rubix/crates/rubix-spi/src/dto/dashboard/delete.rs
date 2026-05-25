//! `rubix.dashboard.delete` — request/response DTOs and tool descriptor.
//!
//! Write verb. Reads the live row for `(tenant_id, page_id)`,
//! refuses any row whose `created_by` is
//! [`crate::dashboard::BUNDLED_PRINCIPAL`] with a [`Diagnostic`]
//! keyed `rubix.dashboard.delete.refused_system`, otherwise calls
//! [`crate::dashboard::DashboardStore::mark_superseded`] which
//! supersedes *every* live revision for the page (the history
//! rows stay so undo and audit work). See
//! `rubix/docs/scope/dashboards/04-tools.md`.
//!
//! `Reversible`: the tool body records an `Op::Delete`
//! `ChangeDraft` whose `before` payload is the full prior body
//! snapshot, so `undo.last` re-inserts the row byte-for-byte.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.delete`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteDashboardRequest {
    /// Owning tenant.
    pub tenant_id: String,
    /// SDUI page id to delete.
    pub page_id: String,
    /// Principal performing the delete (for audit).
    pub deleted_by: String,
}

/// Tool reply for `rubix.dashboard.delete`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteDashboardResponse {
    /// Outcome (`rubix.dashboard.deleted`).
    pub summary: Diagnostic,
    /// Stable SDUI page id (echoed).
    pub page_id: String,
    /// Tenant the row belonged to.
    pub tenant_id: String,
    /// Revision id of the row that was live at delete time
    /// (the row the undo path will re-insert).
    pub prior_revision_id: String,
    /// Number of live rows that were superseded (always `1` on
    /// the happy path because the store keeps at most one head
    /// per `(tenant_id, page_id)`).
    pub superseded: u64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.delete";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Soft-delete a dashboard page (every live revision superseded).",
    when_to_use: concat!(
        "Use when an operator removes a custom dashboard from the ",
        "route table — history rows are retained so the undo verb ",
        "can restore the page byte-for-byte."
    ),
    when_not_to_use: concat!(
        "Do not use against bundled (system-owned) pages — those ",
        "are refused with `rubix.dashboard.delete.refused_system`. ",
        "Do not use to make a destructive edit — that is the role ",
        "of rubix.dashboard.update / rubix.dashboard.page_set."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"tenant-a\", ",
        "\"page_id\": \"dashboard.ops\", \"deleted_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.deleted\", ",
        "\"params\": { \"page_id\": \"dashboard.ops\" } }, ",
        "\"page_id\": \"dashboard.ops\", \"superseded\": 1 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.update",
            wins_when: "the caller wants to mutate the page rather than retire it.",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to reverse a previous `rubix.dashboard.delete` call.",
        },
    ],
};
