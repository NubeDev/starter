//! `rubix.dashboard.update` — request/response DTOs and tool descriptor.
//!
//! Write verb with optimistic concurrency. When the caller supplies
//! an `expected_revision_id` that does not match the row currently
//! live for `(tenant_id, page_id)`, the verb refuses with a
//! [`Diagnostic`] keyed `rubix.dashboard.update.conflict` (the
//! transport layer maps this to HTTP 409). On success the verb
//! inserts a new revision (which the store supersedes the prior
//! head with in the same transaction) and emits
//! `rubix.dashboard.updated`. See
//! `rubix/docs/scope/dashboards/04-tools.md`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.update`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDashboardRequest {
    /// Owning tenant.
    pub tenant_id: String,
    /// SDUI page id to update.
    pub page_id: String,
    /// Optimistic-concurrency token. When `Some`, the verb refuses
    /// with `rubix.dashboard.update.conflict` if the live revision
    /// no longer matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision_id: Option<String>,
    /// Replacement title. When `None`, the prior title is kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Replacement tag list. When `None`, the prior tags are kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// Replacement body — a [`starter_ui_ir::ComponentTree`]
    /// serialised to JSON.
    pub body_json: serde_json::Value,
    /// Principal authoring this revision (for audit).
    pub created_by: String,
}

/// Tool reply for `rubix.dashboard.update`.
///
/// On the conflict path the `revision_id` is the *current* live
/// revision (i.e. the one the caller's `expected_revision_id`
/// failed to match) so the UI can re-fetch and rebase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateDashboardResponse {
    /// Outcome (`rubix.dashboard.updated` on success or
    /// `rubix.dashboard.update.conflict` on stale revision).
    pub summary: Diagnostic,
    /// Stable SDUI page id (echoed).
    pub page_id: String,
    /// Revision id of the row that is currently live: the newly
    /// inserted row on success, or the unchanged live row on
    /// conflict.
    pub revision_id: String,
    /// Tenant that owns the row.
    pub tenant_id: String,
    /// Whether the verb wrote a new revision (`true`) or refused
    /// (`false`).
    pub written: bool,
    /// The `body_json` of the row that was superseded by this
    /// write, if any. Carried in the response so the changelog
    /// recorder can capture a byte-exact `before` snapshot for
    /// `Op::Update` without a follow-up store round-trip.
    /// `None` on a brand-new page (no prior row) or on the
    /// conflict / not-found paths (no write happened).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_body_json: Option<serde_json::Value>,
    /// The `title` of the superseded row. Paired with
    /// [`Self::prior_body_json`] so the `change_for` snapshot can
    /// record the metadata that was live before the write — undo
    /// of a rename then restores the old title instead of
    /// inheriting the new one. `None` whenever `prior_body_json`
    /// is `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_title: Option<String>,
    /// The `tags` of the superseded row. Same rationale as
    /// [`Self::prior_title`]: undo of a re-tag restores the prior
    /// tag set rather than the post-update one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_tags: Option<Vec<String>>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.edit";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Update an existing dashboard page with optimistic concurrency.",
    when_to_use: concat!(
        "Use when a human operator edits a page in the route table — ",
        "pass the `expected_revision_id` you fetched so concurrent ",
        "edits surface as a 409 rather than silently clobber."
    ),
    when_not_to_use: concat!(
        "Do not use from an AI builder — the LLM will not have a ",
        "fresh `expected_revision_id`; call rubix.dashboard.page_set ",
        "instead. Do not use to create a brand-new page — that is ",
        "rubix.dashboard.create."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"tenant-a\", ",
        "\"page_id\": \"dashboard.ops\", ",
        "\"expected_revision_id\": \"...\", ",
        "\"body_json\": { \"ir_version\": 1, \"root\": {} }, ",
        "\"created_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.updated\", ",
        "\"params\": { \"page_id\": \"dashboard.ops\" } }, ",
        "\"page_id\": \"dashboard.ops\", \"revision_id\": \"...\", ",
        "\"written\": true }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.page_set",
            wins_when: "the writer is an AI builder without a fresh `expected_revision_id`.",
        },
        SiblingTool {
            id: "rubix.dashboard.create",
            wins_when: "the page does not yet exist.",
        },
    ],
};
