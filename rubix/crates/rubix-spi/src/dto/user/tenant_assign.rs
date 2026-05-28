//! `rubix.user.tenant.assign` — request/response DTOs and tool
//! descriptor.
//!
//! Assigns (or unassigns) a tenant on an existing user row. The
//! verb is idempotent — a second call with a `tenant_id` matching
//! the stored value returns the `rubix.user.tenant.unchanged`
//! diagnostic and produces *no* `ChangeDraft` (so undo cannot
//! accidentally rewrite an assignment the operator did not flip).
//!
//! Three emission paths cover the operator-visible distinction
//! between "assign" and "unassign":
//!
//! - `rubix.user.tenant.assigned`   — `tenant_id = Some(t)` and
//!   the row changed.
//! - `rubix.user.tenant.unassigned` — `tenant_id = None` and the
//!   row changed (was previously `Some(_)`).
//! - `rubix.user.tenant.unchanged`  — no-op (the stored value
//!   already matches the request).
//!
//! Snapshot shape: `Op::Update`, full `UserRow` on both sides.
//! Every identity-bearing field rides on the response so the
//! `change_for` adapter reconstructs the snapshot byte-exact
//! without a follow-up store read — same posture as `role_set.rs`
//! and `prefs_set.rs` (proposal §3.1 bug-class avoidance).
//!
//! FK posture: the verb validates that `tenant_id`, when `Some`,
//! resolves in [`crate::tenant`]'s store before writing. Silently
//! assigning a user to a nonexistent tenant would be a footgun.
//! Unassignment (`tenant_id = null`) skips the FK check by
//! definition.
//!
//! Cascade-on-tenant-delete: out of scope. There is no
//! `rubix.tenant.delete` verb today. When one lands, the operator-
//! visible decision is whether to (a) refuse delete while users
//! are assigned, (b) cascade-unassign, or (c) block at the FK.
//! Recorded here so the decision gets debated rather than
//! implicitly made.
//!
//! Audit posture: tenant assignment is security-relevant (it
//! controls per-tenant data visibility). The seeded
//! `changelog_kind_policy` pins the whole `user` kind to keep-
//! forever, so this verb's audit row survives the per-kind sweep
//! indefinitely — same as `role.set`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.tenant.assign`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing both
/// is accepted; `user_id` wins. `tenant_id` is `Option<String>`:
///
/// - `Some(id)` → assign the user to that tenant (id must
///   resolve in the tenant store; otherwise the verb fails with
///   `Error::NotFound`).
/// - `None` → unassign the user (clear the tenant link).
///
/// The empty string is NOT a synonym for `None` — an empty
/// `tenant_id` is rejected as `Error::Invalid` so the difference
/// between "explicitly clear" and "accidentally blank" stays
/// visible at the API boundary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserTenantAssignRequest {
    /// Stable user id (preferred). When `None`, the verb resolves
    /// the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user whose tenant to assign.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Tenant id to assign, or `null` to unassign. `serde(default)`
    /// keeps callers that omit the field reading as `None`
    /// (unassign) — the omission is explicit at the wire layer.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserTenantAssignResponse {
    /// Outcome — `rubix.user.tenant.assigned`,
    /// `rubix.user.tenant.unassigned`, or
    /// `rubix.user.tenant.unchanged`.
    pub summary: Diagnostic,
    /// Stable id of the row that was (or already was on this
    /// assignment) updated.
    pub user_id: String,
    /// Email of the row that was updated.
    pub email: String,
    /// The tenant id on the row **before** this call. `None` when
    /// unassigned. When `was_unchanged` is `true`, this equals
    /// `new_tenant_id`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_tenant_id: Option<String>,
    /// The tenant id on the row **after** this call. `None` when
    /// the verb unassigned (or the row was already unassigned).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_tenant_id: Option<String>,
    /// `true` when the row already carried `new_tenant_id` on
    /// entry — the verb is idempotent and no audit row is
    /// recorded.
    pub was_unchanged: bool,
    /// Role carried by the row at the time of the assignment.
    /// Echoed so `change_for` reconstructs the full snapshot
    /// byte-exact (same posture as `prefs_set.role`).
    pub role: String,
    /// Disabled-at timestamp carried by the row at the time of
    /// the assignment. Echoed for the same reason as `role`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
    /// Prefs blob carried by the row at the time of the
    /// assignment. Echoed for the same reason as `role`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs_json: Option<serde_json::Value>,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same permission as the rest of the user-admin verbs — tenant
/// assignment is an operator action sitting alongside role and
/// disable. A future split into a `users.tenant` permission can
/// land when tenant assignment becomes more contentious; not
/// worth the extra surface today.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Assign (or unassign) the tenant on an existing user row.",
    when_to_use: concat!(
        "Use when an operator says \"move Ada to tenant acme\", ",
        "\"unassign bob from his current tenant\", or when a flow ",
        "reassigns users as part of an organisation migration."
    ),
    when_not_to_use: concat!(
        "Do not use to create a tenant (no rubix.tenant.create verb ",
        "exists yet). Do not use to change role or login state ",
        "(use rubix.user.role.set or rubix.user.disable)."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\", \"tenant_id\": \"t-acme\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.tenant.assigned\", ",
        "\"params\": { \"email\": \"ada@example.com\", \"tenant\": \"t-acme\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"prior_tenant_id\": null, \"new_tenant_id\": \"t-acme\", ",
        "\"was_unchanged\": false, \"role\": \"reader\" }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a tenant assignment they just performed.",
        },
        SiblingTool {
            id: "rubix.tenant.list",
            wins_when: "the caller wants to DISCOVER which tenant ids exist before assigning.",
        },
    ],
};
