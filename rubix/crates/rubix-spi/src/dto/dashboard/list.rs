//! `rubix.dashboard.list` — request/response DTOs and tool descriptor.
//!
//! Read-only verb: returns every live dashboard for the caller's
//! tenant, optionally filtered by tag-overlap and/or owner. The
//! per-row authz check (`rubix.dashboard:view`) lands with the
//! create/delete verbs in Phase C.2 — this verb returns the raw
//! tenant-scoped set today. See
//! [`rubix-spi::dashboard::DashboardStore::list_active`].

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ListDashboardsRequest {
    /// Tenant whose dashboards to enumerate. Use `"system"` for
    /// bundled pages.
    pub tenant_id: String,
    /// When non-empty, only return rows whose `tags` overlap any
    /// of these tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags_any: Vec<String>,
    /// When set, only return rows whose `owner_principal` matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// One live dashboard as returned by `rubix.dashboard.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummary {
    /// Stable SDUI page id.
    pub page_id: String,
    /// Revision id of the row that is currently live.
    pub revision_id: String,
    /// Human title of the page.
    pub title: String,
    /// Tags attached to the page.
    pub tags: Vec<String>,
    /// Principal who can `edit` / `delete` the page.
    pub owner_principal: String,
    /// Insertion time of the live revision (RFC-3339).
    pub updated_at: String,
}

/// Tool reply for `rubix.dashboard.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListDashboardsResponse {
    /// Outcome (`rubix.dashboard.listed`).
    pub summary: Diagnostic,
    /// Total row count surfaced.
    pub count: usize,
    /// Rows sorted by `page_id` ascending for stable rendering.
    pub items: Vec<DashboardSummary>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.list";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List the dashboards a tenant currently owns.",
    when_to_use: concat!(
        "Use to render the operator-facing route table, or for the ",
        "agent to discover which pages exist before calling get/edit."
    ),
    when_not_to_use: concat!(
        "Do not use to fetch a single page's body — that is the role ",
        "of rubix.dashboard.get."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"system\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.listed\", ",
        "\"params\": { \"count\": 1 } }, \"count\": 1, ",
        "\"items\": [ { \"page_id\": \"dashboard.disk-overview\", ",
        "\"revision_id\": \"...\", \"title\": \"Disk overview\", ",
        "\"tags\": [\"system\"], \"owner_principal\": \"system\", ",
        "\"updated_at\": \"...\" } ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.dashboard.get",
        wins_when: "the caller already knows the page_id and wants its body, not a summary.",
    }],
};
