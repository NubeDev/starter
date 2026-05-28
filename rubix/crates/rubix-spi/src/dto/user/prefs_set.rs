//! `rubix.user.prefs.set` — request/response DTOs and tool descriptor.
//!
//! Replaces the prefs blob on an existing user row. The verb is
//! idempotent — a second call with a blob that matches the stored
//! value returns the `rubix.user.prefs.unchanged` diagnostic and
//! produces *no* `ChangeDraft` (so undo cannot accidentally rewrite
//! prefs the operator did not actually flip).
//!
//! Snapshot shape: `Op::Update`, `before` = the full prior
//! [`crate::dto::user::list::UserListRow`]-shaped `UserRow` (with
//! the old prefs blob, or `None` if no prefs were set), `after` =
//! the same row with the new prefs blob. The `UserReversible`
//! snapshot is the whole row; per-field identity (`disabled_at_ms`,
//! `role`) is echoed on the response so the snapshot reconstructs
//! byte-exact (same posture as `disable.rs` and `role_set.rs`).
//!
//! Audit posture: prefs writes are not security-relevant in the
//! same way role writes are, but they land in the `user` kind's
//! audit floor regardless — the seeded `changelog_kind_policy`
//! pins the whole `user` kind to keep-forever, not individual
//! verbs. An operator who later decides prefs churn is too noisy
//! for the audit table can split the kind, not the policy.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.prefs.set`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing both
/// is accepted; `user_id` wins. `prefs` is stored verbatim — the
/// rubix tools do not interpret the blob (the UI / agent loop
/// does), so any JSON value is accepted. `null` is legal and
/// stored as `Some(Value::Null)`; a follow-up "clear prefs" verb
/// can land if the difference between "explicit null" and "no
/// prefs row" matters for some consumer.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserPrefsSetRequest {
    /// Stable user id (preferred). When `None`, the verb resolves
    /// the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user whose prefs to set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// New prefs blob. Free-form JSON.
    pub prefs: Value,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPrefsSetResponse {
    /// Outcome — `rubix.user.prefs.set` or `rubix.user.prefs.unchanged`.
    pub summary: Diagnostic,
    /// Stable id of the row that was (or already was on this
    /// prefs blob) updated.
    pub user_id: String,
    /// Email of the row that was updated.
    pub email: String,
    /// The prefs blob on the row **before** this call. `None` when
    /// no prefs were set; semantically different from
    /// `Some(Value::Null)` (explicitly cleared). When
    /// `was_unchanged` is `true`, this equals `new_prefs`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_prefs: Option<Value>,
    /// The prefs blob on the row **after** this call. Echoes the
    /// request (or the stored prior value if `was_unchanged`).
    pub new_prefs: Value,
    /// `true` when the row already carried `new_prefs` on entry —
    /// the verb is idempotent and no audit row is recorded.
    pub was_unchanged: bool,
    /// Role carried by the row at the time of the prefs change.
    /// Echoed so `change_for` reconstructs the full snapshot
    /// byte-exact (same posture as `role_set.disabled_at_ms`).
    pub role: String,
    /// Disabled-at timestamp carried by the row at the time of
    /// the prefs change. Echoed for the same reason as `role`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
    /// Tenant assignment carried by the row at the time of the
    /// prefs change. Echoed for the same reason as `role` — undo
    /// of a prefs change must not silently unassign or reassign
    /// the user's tenant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// `starter-authz` permission string the caller must hold.
///
/// Prefs are user-facing settings, but the verb runs as an
/// operator action (the user-admin tool surface). A future split
/// into a self-service "I set my own prefs" path would land a
/// separate permission; not worth the extra surface today.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Replace the prefs blob on an existing user row.",
    when_to_use: concat!(
        "Use when an operator says \"set Ada's locale to es-ES\", ",
        "\"change bob's temperature units to celsius\", or when a ",
        "flow propagates organisation defaults onto user rows."
    ),
    when_not_to_use: concat!(
        "Do not use to change role or login state (use ",
        "rubix.user.role.set or rubix.user.disable). Do not use ",
        "to merge prefs — this verb replaces the blob wholesale; ",
        "a partial-update verb is a separate proposal."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\", ",
        "\"prefs\": { \"locale\": \"es-ES\", \"units\": \"metric\" } }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.prefs.set\", ",
        "\"params\": { \"email\": \"ada@example.com\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"prior_prefs\": null, \"new_prefs\": { ... }, ",
        "\"was_unchanged\": false, \"role\": \"admin\" }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a prefs change they just performed.",
        },
        SiblingTool {
            id: "rubix.user.role.set",
            wins_when: "the caller wants to change the role string, not the prefs blob.",
        },
    ],
};
