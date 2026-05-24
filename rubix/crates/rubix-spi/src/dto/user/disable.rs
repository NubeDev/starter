//! `rubix.user.disable` — request/response DTOs and tool descriptor.
//!
//! See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! for the verb contract and the `Reversible` snapshot shape.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.disable`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing both
/// is accepted; `user_id` wins.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserDisableRequest {
    /// Stable user id (preferred). When `None`, the verb resolves
    /// the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user to disable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDisableResponse {
    /// Outcome — `rubix.user.disabled` or `rubix.user.already_disabled`.
    pub summary: Diagnostic,
    /// Stable id of the row that was (or already was) disabled.
    pub user_id: String,
    /// Email of the row that was disabled.
    pub email: String,
    /// Role of the row that was disabled (echoed so the undo path
    /// can reconstruct the full prior snapshot without a follow-up
    /// read).
    pub role: String,
    /// `true` when the row was already in the disabled state on
    /// entry — the verb is idempotent and reports the prior state.
    pub was_already_disabled: bool,
    /// Epoch milliseconds (UTC) at which disable took effect.
    /// When `was_already_disabled` is `true`, this is the prior
    /// disabled-at timestamp from the row.
    pub disabled_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Disable a user so they cannot log in or be assigned to teams.",
    when_to_use: concat!(
        "Use when an operator says \"disable Ada\", \"deactivate this ",
        "account\", or when a flow is offboarding a user."
    ),
    when_not_to_use: concat!(
        "Do not use to permanently delete a user (row stays for audit). ",
        "Do not use to remove a user from a single team (that is ",
        "rubix.team.unassign, not yet wired)."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.disabled\", ",
        "\"params\": { \"email\": \"ada@example.com\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"was_already_disabled\": false, \"disabled_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a disable they just performed.",
        },
        SiblingTool {
            id: "rubix.user.create",
            wins_when: "the user does not yet exist; create makes a new row.",
        },
    ],
};
