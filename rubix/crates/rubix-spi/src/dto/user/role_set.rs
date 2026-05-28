//! `rubix.user.role.set` — request/response DTOs and tool descriptor.
//!
//! Sets the role on an existing user row. The verb is idempotent —
//! a second call with the same role returns the
//! `rubix.user.role.unchanged` diagnostic and produces *no*
//! `ChangeDraft` (so undo cannot accidentally rewrite a role that
//! was never changed).
//!
//! Snapshot shape: `Op::Update`, `before` = the full prior
//! [`crate::dto::user::list::UserListRow`]-shaped `UserRow` (with the
//! old role), `after` = the same row with the new role. The
//! `UserReversible::apply_inverse` path replays the whole snapshot,
//! so undo of a role change rewinds nothing else — every other
//! field round-trips byte-exact.
//!
//! Audit posture: role writes are security-relevant. The boot-time
//! [`changelog_kind_policy`] migration in
//! `rubix-store-postgres/migrations/changelog_policy/` pins the
//! `user` kind to `max_age_days = NULL` (keep forever) so the
//! `Change` row produced by this verb survives the per-kind sweep
//! indefinitely. See `rubix/docs/proposal/audit-log.md`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.role.set`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing both
/// is accepted; `user_id` wins. `role` is the target role string —
/// today the user-admin verbs treat role as an opaque string
/// (`reader` / `writer` / `admin` by convention), so the validator
/// is "non-empty" rather than an enum. A strict enum lands when the
/// authz role model formalises.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserRoleSetRequest {
    /// Stable user id (preferred). When `None`, the verb resolves
    /// the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user whose role to set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// New role string. Non-empty; trimmed leading/trailing
    /// whitespace is rejected as `Error::Invalid`.
    pub role: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserRoleSetResponse {
    /// Outcome — `rubix.user.role.set` or `rubix.user.role.unchanged`.
    pub summary: Diagnostic,
    /// Stable id of the row that was (or already was on this role)
    /// updated.
    pub user_id: String,
    /// Email of the row that was updated (echoed so the agent loop
    /// does not need a follow-up read to log the change).
    pub email: String,
    /// The role on the row **before** this call. When `was_unchanged`
    /// is `true`, this equals `new_role`.
    pub prior_role: String,
    /// The role on the row **after** this call.
    pub new_role: String,
    /// `true` when the row already carried `new_role` on entry — the
    /// verb is idempotent and no audit row is recorded.
    pub was_unchanged: bool,
    /// Disabled-at timestamp carried by the row at the time of the
    /// role change. Echoed so `change_for` reconstructs the full
    /// snapshot byte-exact — without this, undo of a role flip on
    /// a disabled user would silently re-enable them. See the
    /// dashboard rename fix (proposal §3.1) for the prior bug
    /// class this prevents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
    /// Prefs blob carried by the row at the time of the role
    /// change. Echoed for the same reason as `disabled_at_ms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs_json: Option<serde_json::Value>,
    /// Tenant assignment carried by the row at the time of the
    /// role change. Echoed for the same reason as `prefs_json` —
    /// undo of a role flip on a tenant-assigned user must not
    /// silently unassign them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same permission as the rest of the user-admin verbs — operators
/// who can disable a user can also change their role. Future work
/// may split this into a `users.role` permission once role writes
/// become more contentious; not worth the extra surface today.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Set the role on an existing user row.",
    when_to_use: concat!(
        "Use when an operator says \"promote Ada to admin\", ",
        "\"demote bob to reader\", or when a flow reassigns roles ",
        "as part of an onboarding step."
    ),
    when_not_to_use: concat!(
        "Do not use to create a user (that is rubix.user.create). ",
        "Do not use to disable a user (that is rubix.user.disable — ",
        "role change does not affect login)."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\", \"role\": \"admin\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.role.set\", ",
        "\"params\": { \"email\": \"ada@example.com\", \"prior\": ",
        "\"reader\", \"new\": \"admin\" } }, \"user_id\": \"u-...\", ",
        "\"email\": \"ada@example.com\", \"prior_role\": \"reader\", ",
        "\"new_role\": \"admin\", \"was_unchanged\": false }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a role change they just performed.",
        },
        SiblingTool {
            id: "rubix.user.disable",
            wins_when: "the caller wants to BLOCK login, not change the role string.",
        },
    ],
};
