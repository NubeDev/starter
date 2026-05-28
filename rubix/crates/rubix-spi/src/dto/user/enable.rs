//! `rubix.user.enable` — request/response DTOs and tool descriptor.
//!
//! Inverse of [`crate::user::disable`]. Clears `disabled_at_ms`
//! back to `None`. Idempotent — calling enable on an
//! already-enabled user returns the
//! `rubix.user.already_enabled` diagnostic and produces no
//! `ChangeDraft` (mirrors disable's `was_already_disabled` posture).
//!
//! Why a dedicated verb rather than "just undo"? `rubix.undo.last`
//! is per-actor; an operator can only undo their own most recent
//! mutation. If admin A disables Ada and admin B later wants to
//! re-enable her, B has no path through undo. `user.enable` is
//! the canonical re-enable verb. Operators can still undo their
//! own disable, but the surface no longer depends on it.
//!
//! Snapshot shape: `Op::Update`, `before` = the prior [`UserRow`]
//! (with `disabled_at_ms = Some(...)`), `after` = the same row
//! with `disabled_at_ms = None`. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.enable`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing both
/// is accepted; `user_id` wins.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserEnableRequest {
    /// Stable user id (preferred). When `None`, the verb resolves
    /// the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user to re-enable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Tool reply.
///
/// Echoes every identity-bearing field of the row so
/// [`ReversibleTool::change_for`] reconstructs the full snapshot
/// byte-exact (proposal §3.1 fix).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserEnableResponse {
    /// Outcome — `rubix.user.enabled` or `rubix.user.already_enabled`.
    pub summary: Diagnostic,
    /// Stable id of the row that was (or already was) enabled.
    pub user_id: String,
    /// Email of the row.
    pub email: String,
    /// Role of the row (echoed for snapshot reconstruction).
    pub role: String,
    /// Prefs blob carried by the row (echoed for snapshot
    /// reconstruction — undo of enable must restore the prior
    /// prefs as well as the prior disabled state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs_json: Option<serde_json::Value>,
    /// Tenant assignment carried by the row (echoed for snapshot
    /// reconstruction).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// `true` when the row was already in the enabled state on
    /// entry — the verb is idempotent and reports the prior state.
    pub was_already_enabled: bool,
    /// The `disabled_at_ms` value the row carried at the time of
    /// the enable call. `None` when `was_already_enabled` is
    /// `true`; otherwise `Some(prior_ts)`. Echoed so `change_for`
    /// reconstructs the `before` snapshot byte-exact (without it,
    /// undo of enable would restore `disabled_at_ms = Some(now())`
    /// instead of the original timestamp).
    pub prior_disabled_at_ms: Option<i64>,
    /// Epoch milliseconds (UTC) at which the enable took effect.
    pub enabled_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same `users.write` permission as `disable` — both are
/// account-state lifecycle verbs and they share an authorisation
/// boundary.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Re-enable a previously disabled user.",
    when_to_use: concat!(
        "Use when an operator says \"re-enable Ada\", \"reactivate this ",
        "account\", or when an offboarding was reverted manually. This ",
        "is the canonical enable surface \u{2014} use it instead of relying on ",
        "rubix.undo.last when another actor performed the disable."
    ),
    when_not_to_use: concat!(
        "Do not use to create a new user (that is rubix.user.create). ",
        "Do not use to change a role (that is rubix.user.role.set). ",
        "Do not use to disable a user (that is rubix.user.disable)."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.enabled\", ",
        "\"params\": { \"email\": \"ada@example.com\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"was_already_enabled\": false, \"prior_disabled_at_ms\": ",
        "1764800000000, \"enabled_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.user.disable",
            wins_when: "the caller wants to disable, not re-enable, the account.",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to reverse their OWN most recent mutation; enable is the cross-actor surface.",
        },
    ],
};
