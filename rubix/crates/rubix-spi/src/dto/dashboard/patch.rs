//! `rubix.dashboard.patch` — request/response DTOs and tool descriptor.
//!
//! Partial-update verb. Applies an RFC 6902 JSON-Patch (a JSON array
//! of `{op, path, value?}` operations) to the live `body_json`, then
//! routes the synthesised body through the same `insert_revision`
//! path `rubix.dashboard.update` uses — so the changelog still
//! records the full before/after snapshot the undo path expects and
//! a `patch` is byte-exact-reversible just like a full `update`.
//!
//! Optimistic concurrency: when the caller supplies an
//! `expected_revision_id` that does not match the row currently live
//! for `(tenant_id, page_id)`, the verb refuses with a [`Diagnostic`]
//! keyed `rubix.dashboard.patch.conflict` (transport layer maps to
//! HTTP 409). A malformed patch (bad path, missing required field,
//! wrong target type) yields `rubix.dashboard.patch.invalid`. On
//! success the verb emits `rubix.dashboard.patched`.
//!
//! The `patch` field is left as a free-form [`serde_json::Value`] so
//! `rubix-spi` stays contracts-only (no `json-patch` crate dep here);
//! the tool implementation in `rubix-tools` deserialises it into a
//! concrete `json_patch::Patch` and surfaces any structural error as
//! `rubix.dashboard.patch.invalid`.
//!
//! See `rubix/docs/scope/dashboards/04-tools.md` and
//! `rubix/docs/design/sdui/dashboard-api-usage.md` issue #4.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.dashboard.patch`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatchDashboardRequest {
    /// Owning tenant.
    pub tenant_id: String,
    /// SDUI page id to patch.
    pub page_id: String,
    /// Optimistic-concurrency token. When `Some`, the verb refuses
    /// with `rubix.dashboard.patch.conflict` if the live revision no
    /// longer matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_revision_id: Option<String>,
    /// RFC 6902 JSON-Patch document — a JSON array of operations.
    /// Each element is `{op, path, value?, from?}`; supported `op`s
    /// are `add` / `remove` / `replace` / `move` / `copy` / `test`.
    /// Carried as an open `Value` here so `rubix-spi` stays free of
    /// the `json-patch` crate dep; the tool deserialises it.
    #[schema(value_type = Object)]
    pub patch: serde_json::Value,
    /// Principal authoring this revision (for audit).
    pub created_by: String,
}

/// Tool reply for `rubix.dashboard.patch`.
///
/// On the conflict path the `revision_id` is the *current* live
/// revision (i.e. the one the caller's `expected_revision_id`
/// failed to match) so the UI can re-fetch and rebase.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PatchDashboardResponse {
    /// Outcome (`rubix.dashboard.patched` on success,
    /// `rubix.dashboard.patch.conflict` on stale revision, or
    /// `rubix.dashboard.patch.invalid` on a malformed patch).
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
    /// The full post-patch `body_json` that landed in the new
    /// revision. Carried in the response so the changelog
    /// recorder can capture a byte-exact `after` snapshot for
    /// `Op::Update` without re-fetching the row — patch undo is
    /// therefore strictly precise. Omitted when `written = false`
    /// (conflict / invalid).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_json: Option<serde_json::Value>,
    /// The `body_json` of the row that was superseded by this
    /// patch. Paired with [`Self::body_json`], the recorder gets
    /// a byte-exact `before` and `after` for the changelog row.
    /// `None` on the conflict / not-found / invalid-patch paths
    /// (no write happened).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_body_json: Option<serde_json::Value>,
    /// The `title` of the superseded row. Carried so the
    /// `change_for` snapshot can record the metadata live before
    /// the patch — patch never mutates title or tags, but the
    /// snapshot still needs them so the undo path doesn't write
    /// an empty title back into the row.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_title: Option<String>,
    /// The `tags` of the superseded row. Same rationale as
    /// [`Self::prior_title`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_tags: Option<Vec<String>>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "rubix.dashboard.edit";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Apply an RFC 6902 JSON-Patch to a dashboard page's body_json.",
    when_to_use: concat!(
        "Use for programmatic edits that touch a small fragment of ",
        "`body_json` — e.g. flipping a single widget's `source` or ",
        "inserting one row — when shipping the whole body would be ",
        "wasteful. Pass the `expected_revision_id` you fetched so ",
        "concurrent edits surface as a 409 rather than silently ",
        "clobber."
    ),
    when_not_to_use: concat!(
        "Do not use to replace the body wholesale — call ",
        "`rubix.dashboard.update` with the new body instead; the ",
        "changelog snapshot is identical and the request is easier ",
        "to read. Do not use to mutate a flow slot value at runtime ",
        "— that is `rubix.dashboard.page_set`. Do not use to create ",
        "a brand-new page — that is `rubix.dashboard.create`."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"tenant-a\", ",
        "\"page_id\": \"dashboard.ops\", ",
        "\"expected_revision_id\": \"...\", ",
        "\"patch\": [ { \"op\": \"replace\", ",
        "\"path\": \"/root/children/0/title\", ",
        "\"value\": \"Ops (live)\" } ], ",
        "\"created_by\": \"alice\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.dashboard.patched\", ",
        "\"params\": { \"page_id\": \"dashboard.ops\" } }, ",
        "\"page_id\": \"dashboard.ops\", \"revision_id\": \"...\", ",
        "\"written\": true }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.dashboard.update",
            wins_when: "the caller is replacing the body wholesale.",
        },
        SiblingTool {
            id: "rubix.dashboard.page_set",
            wins_when: "the writer is mutating a flow slot value, not the page IR.",
        },
        SiblingTool {
            id: "rubix.dashboard.create",
            wins_when: "the page does not yet exist.",
        },
    ],
};
