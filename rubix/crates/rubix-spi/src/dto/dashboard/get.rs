//! `rubix.dashboard.get` — request/response DTOs and tool descriptor.
//!
//! Read-only verb: returns the single live revision for
//! `(tenant_id, page_id)`, or a structured `rubix.dashboard.get.not_found`
//! diagnostic on miss. The resolver caches; this verb sends no
//! caching headers itself. See
//! [`rubix-spi::dashboard::DashboardStore::get_active`].

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.get`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetDashboardRequest {
    /// Owning tenant (use `"system"` for bundled pages).
    pub tenant_id: String,
    /// SDUI page id (`dashboard.<slug>`).
    pub page_id: String,
}

/// Tool reply for `rubix.dashboard.get`.
///
/// The `body_json` field is the resolved
/// [`starter_ui_ir::ComponentTree`] kept as [`serde_json::Value`]
/// so `rubix-spi` stays IR-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetDashboardResponse {
    /// Outcome (`rubix.dashboard.fetched` on success, or
    /// `rubix.dashboard.get.not_found` when `body_json` is absent).
    pub summary: Diagnostic,
    /// Stable SDUI page id (echoed for convenience).
    pub page_id: String,
    /// Revision id of the row returned. Absent when the page does
    /// not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_id: Option<String>,
    /// Tenant that owns the row. Absent when the page does not
    /// exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Principal who can `edit` / `delete`. Absent when the page
    /// does not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_principal: Option<String>,
    /// Human title of the page. Absent on miss.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Tags attached to the page. Empty on miss.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Wire body (a `starter_ui_ir::ComponentTree`). Absent on
    /// miss.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_json: Option<serde_json::Value>,
    /// Principal that authored the revision. Absent on miss.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    /// Insertion time (RFC-3339). Absent on miss.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.view";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Fetch the active body of a dashboard page.",
    when_to_use: concat!(
        "Use to load a single dashboard page by id for inspection ",
        "or for the SDUI resolver to render."
    ),
    when_not_to_use: concat!(
        "Do not use to enumerate every dashboard — that is the role ",
        "of rubix.dashboard.list. Do not use to walk revisions — ",
        "that is rubix.dashboard.history."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"system\", ",
        "\"page_id\": \"dashboard.disk-overview\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.fetched\", ",
        "\"params\": { \"page_id\": \"dashboard.disk-overview\" } }, ",
        "\"page_id\": \"dashboard.disk-overview\", ",
        "\"revision_id\": \"...\", \"body_json\": { ... } }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.list",
            wins_when: "the caller wants every dashboard, not one by id.",
        },
        SiblingTool {
            id: "rubix.dashboard.history",
            wins_when: "the caller wants every revision of a single page, not just the live one.",
        },
    ],
};
