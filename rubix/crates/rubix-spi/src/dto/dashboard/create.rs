//! `rubix.dashboard.create` — request/response DTOs and tool descriptor.
//!
//! Write verb. Inserts the first revision of a new SDUI page via
//! [`rubix-spi::dashboard::DashboardStore::insert_revision`] and
//! re-asserts the `rubix.dashboard.page` `ResourceSpec` on the
//! authz registry. Returns a [`Diagnostic`] keyed
//! `rubix.dashboard.created` on success or
//! `rubix.dashboard.create.duplicate_id` on a name clash (HTTP 409).
//! See `rubix/docs/scope/dashboards/04-tools.md`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.create`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDashboardRequest {
    /// Owning tenant (use `"system"` for bundled pages).
    pub tenant_id: String,
    /// Stable SDUI page id (`dashboard.<slug>`). Must match the
    /// shape validated by the tool body — `^dashboard\.[a-z0-9-]+$`.
    pub page_id: String,
    /// Principal who can later `edit` / `delete` the page.
    pub owner_principal: String,
    /// Human title for the route table.
    pub title: String,
    /// Free-form tag list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Wire body — a [`starter_ui_ir::ComponentTree`] serialised to
    /// JSON. The DTO stays IR-agnostic (kept as
    /// [`serde_json::Value`]).
    pub body_json: serde_json::Value,
    /// Principal that authored this revision (for audit). Often
    /// equals `owner_principal`.
    pub created_by: String,
}

/// Tool reply for `rubix.dashboard.create`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDashboardResponse {
    /// Outcome (`rubix.dashboard.created`).
    pub summary: Diagnostic,
    /// Stable SDUI page id (echoed for convenience).
    pub page_id: String,
    /// Revision id of the row just inserted.
    pub revision_id: String,
    /// Tenant that owns the row.
    pub tenant_id: String,
    /// Principal who can `edit` / `delete`.
    pub owner_principal: String,
    /// Human title of the page.
    pub title: String,
    /// Tags attached to the page.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Principal that authored the revision.
    pub created_by: String,
    /// Insertion time (RFC-3339).
    pub created_at: String,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.create";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Create a new dashboard page.",
    when_to_use: concat!(
        "Use to author a fresh SDUI page from scratch — supply the ",
        "stable `page_id`, the title, and the resolved ComponentTree ",
        "body."
    ),
    when_not_to_use: concat!(
        "Do not use to mutate an existing page — that is the role of ",
        "rubix.dashboard.update / rubix.dashboard.page_set. Do not use ",
        "to clone an existing page — that is rubix.dashboard.duplicate."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"tenant-a\", ",
        "\"page_id\": \"dashboard.ops\", ",
        "\"owner_principal\": \"alice\", \"title\": \"Ops\", ",
        "\"body_json\": { \"ir_version\": 1, \"root\": {} }, ",
        "\"created_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.created\", ",
        "\"params\": { \"title\": \"Ops\" } }, ",
        "\"page_id\": \"dashboard.ops\", \"revision_id\": \"...\" }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.update",
            wins_when: "the caller wants to mutate an existing page, not create one.",
        },
        SiblingTool {
            id: "rubix.dashboard.duplicate",
            wins_when:
                "the caller wants to clone an existing page rather than author from scratch.",
        },
    ],
};
