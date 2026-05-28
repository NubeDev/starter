//! `rubix.tenant.update` — request/response DTOs and tool descriptor.
//!
//! Mutates the human-facing fields of an existing tenant row —
//! `name` and/or `locale`. The id is immutable; renaming the
//! `tenant_id` would invalidate every FK in the system
//! (`UserRow.tenant_id`, every `Change.resource.tenant`, every
//! per-tenant warehouse view). If the operator wants a different
//! id, the correct shape is `tenant.create` + reassign + `tenant.delete`,
//! not a rename.
//!
//! Idempotency: a call where the supplied `name`/`locale` already
//! match the stored row returns the `rubix.tenant.unchanged`
//! diagnostic and produces *no* `ChangeDraft` — same posture as
//! [`crate::user::role_set`] / [`crate::user::tenant_assign`].
//! A call with all fields omitted is rejected as
//! `Error::Invalid` (not silently treated as unchanged): a no-op
//! request is almost always a wire-shaped bug.
//!
//! Snapshot shape: `Op::Update`, `before` = the full prior
//! [`crate::tenant::store::TenantRow`], `after` = the row with the
//! new fields applied. [`TenantReversible::apply_inverse`] replays
//! the whole snapshot — undo of a rename rewinds the locale too
//! if both changed, which is correct: the operator either
//! intended both flips together or they should have issued two
//! separate verbs.
//!
//! Audit posture: see [`crate::tenant::create`] — the seed in
//! `changelog_policy/0002_*.sql` pins the `tenant` kind to
//! `max_age_days = NULL`, so update rows persist past undo
//! retention.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.tenant.update`.
///
/// `tenant_id` is required (the row to mutate). `name` and
/// `locale` are both optional; at least one of them must be
/// `Some(...)` — the verb refuses an "update with no fields"
/// request. Field-level absence (`None`) means "leave this field
/// alone"; an explicit empty string is rejected.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TenantUpdateRequest {
    /// Id of the tenant row to mutate. Required.
    pub tenant_id: String,
    /// New human-facing name. When `Some`, must be non-empty,
    /// trimmed, and unique across all *other* tenant rows.
    /// `None` leaves the name unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New IETF locale tag. When `Some`, must be non-empty and
    /// trimmed. `None` leaves the locale unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Tool reply.
///
/// Echoes every identity-bearing field of the post-update row so
/// `change_for` reconstructs the full `before`/`after` snapshot
/// byte-exact without a follow-up store read (proposal §3.1 fix).
/// `prior_name` and `prior_locale` carry the pre-update values
/// so the snapshot's `before` field is reconstructible from the
/// response alone.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantUpdateResponse {
    /// Outcome — `rubix.tenant.updated` or
    /// `rubix.tenant.unchanged`.
    pub summary: Diagnostic,
    /// Stable id (echoed; immutable).
    pub tenant_id: String,
    /// Human-facing name as of pre-update.
    pub prior_name: String,
    /// Human-facing name as of post-update (== `prior_name` when
    /// `name` was not requested or matched).
    pub new_name: String,
    /// Locale as of pre-update.
    pub prior_locale: String,
    /// Locale as of post-update (== `prior_locale` when `locale`
    /// was not requested or matched).
    pub new_locale: String,
    /// `true` when both requested fields (if any) already matched
    /// the stored row. When `true`, [`ReversibleTool::change_for`]
    /// returns `None`.
    pub was_unchanged: bool,
    /// Epoch milliseconds (UTC) at which the update took effect
    /// (or would have, on the unchanged path).
    pub updated_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same `tenants.write` permission as `tenant.create` /
/// `tenant.delete` — all three are tenant-lifecycle verbs and
/// they share an authorisation boundary.
pub const REQUIRED_PERMISSION: &str = "tenants.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Rename a tenant or change its locale (id stays the same).",
    when_to_use: concat!(
        "Use when the operator says \"rename tenant t-acme to Acme Corp\" ",
        "or \"set tenant t-acme locale to es\". At least one of name / ",
        "locale must be supplied; field-level None leaves the field alone."
    ),
    when_not_to_use: concat!(
        "Do not use to change a tenant id — ids are immutable; renaming ",
        "an id would invalidate every per-tenant FK. Do not use to ",
        "assign a user to a tenant (that is rubix.user.tenant.assign). ",
        "Do not use to provision a new tenant (that is rubix.tenant.create)."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"t-acme\", \"name\": \"Acme Corp\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.tenant.updated\", ",
        "\"params\": { \"tenant\": \"t-acme\", \"prior\": \"Acme\", ",
        "\"new\": \"Acme Corp\" } }, \"tenant_id\": \"t-acme\", ",
        "\"prior_name\": \"Acme\", \"new_name\": \"Acme Corp\", ",
        "\"prior_locale\": \"en\", \"new_locale\": \"en\", ",
        "\"was_unchanged\": false, \"updated_at_ms\": ... }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a tenant update they just performed.",
        },
        SiblingTool {
            id: "rubix.tenant.create",
            wins_when: "the caller wants to PROVISION a new tenant, not rename an existing one.",
        },
        SiblingTool {
            id: "rubix.tenant.delete",
            wins_when: "the caller wants to REMOVE the tenant, not rename it.",
        },
    ],
};
