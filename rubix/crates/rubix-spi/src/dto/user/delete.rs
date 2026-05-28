//! `rubix.user.delete` \u{2014} request/response DTOs and tool descriptor.
//!
//! Hard-deletes a user. The verb is **not** silent-idempotent
//! \u{2014} calling delete on a missing id returns
//! `Error::NotFound`. Deletes are operator-visible and the
//! "I thought I already deleted that" question is better
//! answered by an explicit NotFound than by a silent success
//! (same posture as `rubix.tenant.delete`).
//!
//! ## When to reach for this vs `rubix.user.disable`
//!
//! Most operator workflows want **disable**, not delete. Disable
//! keeps the row for audit, can be undone in one step
//! (`rubix.user.enable`), and preserves the user's history. Use
//! delete only when:
//!
//! - The row was created in error (typo'd email, wrong tenant)
//!   and has no operational history worth keeping.
//! - GDPR / right-to-be-forgotten request requires the row to be
//!   removed.
//! - Cleaning up a staging / test account.
//!
//! ## Cascade decision: refuse if member of any team
//!
//! The verb refuses to delete a user that is a member of any
//! team via [`crate::team::TeamRow::members`]. The operator must
//! `rubix.team.member.unassign` from every team first.
//!
//! Mirrors the [`crate::dto::tenant::delete`] cascade decision.
//! The alternatives considered:
//!
//! - **Cascade-unassign across teams** \u{2014} silently remove
//!   the user from every team. Rejected for the same reasons
//!   `rubix.tenant.delete` rejects cascade-unassign: a delete
//!   that touches N team rows produces N audit entries that may
//!   surprise the operator later, and an operator deleting a
//!   user may not own every team the user is on.
//! - **Block at the FK** \u{2014} not available: team membership
//!   is a JSONB map on the team row (see the `rubix_teams`
//!   migration preamble); JSONB keys can't carry FKs, so there
//!   is no DB-level enforcement to fall back on.
//! - **Refuse with a structured diagnostic** (this verb's
//!   choice) \u{2014} the operator sees `rubix.user.in_teams`
//!   with the count of teams blocking the delete, can list them
//!   via `rubix.team.list`, and fixes the underlying state
//!   explicitly.
//!
//! ## Tenant assignment
//!
//! Unlike team membership, `tenant_id` is a column on the user
//! row itself, so it disappears with the row on delete and
//! does NOT block the verb. The undo path restores the tenant
//! assignment byte-exact via the snapshot.
//!
//! Snapshot shape: `Op::Delete`, `before` = the full prior
//! [`crate::user::UserRow`] (so undo can re-create the row
//! including `disabled_at_ms`, `prefs_json`, `tenant_id`),
//! `after = None`. The
//! [`UserReversible::apply_inverse`] path re-puts the row. Note
//! that undo restores the user row but does NOT re-assign team
//! memberships \u{2014} those were removed by separate
//! `rubix.team.member.unassign` calls before the delete and
//! live in their own audit chain. The operator chains the undos
//! in reverse order to fully restore.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.delete`.
///
/// Exactly one of `user_id` or `email` MUST be set. Passing
/// both is accepted; `user_id` wins (mirrors `rubix.user.disable`).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserDeleteRequest {
    /// Stable user id (preferred). When `None`, the verb
    /// resolves the row via `email`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Login email of the user to delete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

/// Tool reply.
///
/// Every identity-bearing field on the deleted row is echoed so
/// the `change_for` snapshot can reconstruct the full prior
/// state byte-exact without re-reading the (now-deleted) row.
/// Same \u{00A7}3.1 echo rule the other user verbs follow.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDeleteResponse {
    /// Outcome \u{2014} `rubix.user.deleted`.
    pub summary: Diagnostic,
    /// Stable id of the row that was deleted.
    pub user_id: String,
    /// Email of the row that was deleted.
    pub email: String,
    /// Role of the row that was deleted.
    pub role: String,
    /// `disabled_at_ms` carried by the row at the time of the
    /// delete (`None` when the row was enabled). Echoed so undo
    /// restores the disabled-or-enabled state byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
    /// Prefs blob carried by the row at the time of the delete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs_json: Option<serde_json::Value>,
    /// Tenant assignment carried by the row at the time of the
    /// delete (`None` when the user was unassigned). Echoed so
    /// undo restores the assignment byte-exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Epoch milliseconds (UTC) at which delete took effect.
    pub deleted_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same `users.write` permission as disable / enable / role.set
/// \u{2014} user lifecycle is a single scope today, not split
/// per-op. A future `users.delete` scope would let a tenant
/// admin disable users but require a separate
/// data-protection-officer role to hard-delete; recorded as a
/// follow-up rather than implicitly adopted here.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Hard-delete a user; refuses if the user is still a member of any team.",
    when_to_use: concat!(
        "Use when GDPR / right-to-be-forgotten requires removing a user, ",
        "when cleaning up a typo'd account that has no operational ",
        "history, or when removing a staging / test account. The ",
        "operator must remove the user from every team first \u{2014} ",
        "the verb returns rubix.user.in_teams when memberships block ",
        "the delete."
    ),
    when_not_to_use: concat!(
        "Do not use to temporarily deactivate a user (that is ",
        "rubix.user.disable \u{2014} keeps the row, preserves history, ",
        "single-step undo via rubix.user.enable). Do not use to remove ",
        "a user from a single team (that is rubix.team.member.unassign)."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.deleted\", ",
        "\"params\": { \"email\": \"ada@example.com\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"role\": \"reader\", \"deleted_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.user.disable",
            wins_when: "the caller wants to deactivate the user temporarily and keep history.",
        },
        SiblingTool {
            id: "rubix.team.member.unassign",
            wins_when: "the caller needs to remove this user from every team before delete will succeed.",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a user delete they just performed (restores the row).",
        },
    ],
};
