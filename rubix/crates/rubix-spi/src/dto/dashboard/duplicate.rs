//! `rubix.dashboard.duplicate` — request/response DTOs and tool descriptor.
//!
//! Write verb. Reads the live revision of the source page via
//! [`crate::dashboard::DashboardStore::get_active`] and inserts a
//! new revision at `target_page_id` whose body matches the
//! source. The new row gets a fresh `revision_id` (the store
//! mints it) and the caller-supplied `new_owner_principal` /
//! `created_by` so the duplicate is operator-owned even when the
//! source was bundled. On a missing source the verb returns a
//! [`Diagnostic`] keyed `rubix.dashboard.duplicate.source_not_found`
//! (transport maps to HTTP 404). See
//! `rubix/docs/scope/dashboards/04-tools.md`.
//!
//! `Reversible`: the tool body records an `Op::Create`
//! `ChangeDraft` whose `after` payload is the newly inserted row,
//! so `undo.last` retires the duplicate via
//! [`crate::dashboard::DashboardStore::mark_superseded`].

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.duplicate`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DuplicateDashboardRequest {
    /// Tenant the source page lives in.
    pub source_tenant_id: String,
    /// SDUI page id to clone.
    pub source_page_id: String,
    /// Tenant the duplicate is written into (commonly the caller's
    /// own tenant).
    pub target_tenant_id: String,
    /// Fresh SDUI page id for the duplicate
    /// (`dashboard.<lowercase-slug>`).
    pub target_page_id: String,
    /// Principal who will own (`edit` / `delete`) the duplicate.
    pub new_owner_principal: String,
    /// Principal that authors the first revision of the duplicate
    /// (for audit).
    pub created_by: String,
    /// Optional title override; falls back to the source title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional tag override; falls back to the source tags.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Tool reply for `rubix.dashboard.duplicate`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DuplicateDashboardResponse {
    /// Outcome (`rubix.dashboard.duplicated`).
    pub summary: Diagnostic,
    /// Source page id (echoed for convenience).
    pub source_page_id: String,
    /// Fresh SDUI page id of the duplicate.
    pub page_id: String,
    /// Revision id minted by the store for the duplicate.
    pub revision_id: String,
    /// Tenant the duplicate lives in.
    pub tenant_id: String,
    /// Owner principal of the duplicate.
    pub owner_principal: String,
    /// Title of the duplicate (echoed; may differ from the source
    /// when the caller supplied a `title` override).
    pub title: String,
    /// Tags on the duplicate (echoed).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Principal that authored the first revision of the
    /// duplicate.
    pub created_by: String,
    /// Insertion time of the duplicate (RFC-3339).
    pub created_at: String,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.create";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Clone an existing dashboard page into a new page id.",
    when_to_use: concat!(
        "Use to start from a bundled or peer-authored dashboard ",
        "and customise it under a new id without mutating the ",
        "source."
    ),
    when_not_to_use: concat!(
        "Do not use to author a page from scratch — that is the ",
        "role of rubix.dashboard.create. Do not use to mutate the ",
        "source page — that is rubix.dashboard.update."
    ),
    example: concat!(
        "Input:  { \"source_tenant_id\": \"system\", ",
        "\"source_page_id\": \"dashboard.disk-overview\", ",
        "\"target_tenant_id\": \"tenant-a\", ",
        "\"target_page_id\": \"dashboard.disk-mine\", ",
        "\"new_owner_principal\": \"alice\", ",
        "\"created_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.duplicated\", ",
        "\"params\": { \"source_page_id\": \"dashboard.disk-overview\", ",
        "\"page_id\": \"dashboard.disk-mine\" } }, ",
        "\"page_id\": \"dashboard.disk-mine\", \"revision_id\": \"...\" }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.create",
            wins_when: "the caller has a ComponentTree in hand rather than wanting to clone an existing page.",
        },
        SiblingTool {
            id: "rubix.dashboard.update",
            wins_when: "the caller wants to mutate the existing page rather than clone it.",
        },
    ],
};
