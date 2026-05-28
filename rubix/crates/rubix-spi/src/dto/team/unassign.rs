//! `rubix.team.unassign` — request/response DTOs and tool descriptor.
//!
//! Removes an existing user from an existing team. Mirrors the
//! shape of [`crate::team::assign`] in reverse: idempotent — if
//! the user is not a member on entry, returns the same code with
//! `already_not_member = true` and produces no `ChangeDraft`.
//!
//! Snapshot shape: `Op::Update`, `before` / `after` carry a sparse
//! [`crate::team::store::TeamPatch`] with only the `members` field
//! populated. Undo through `TeamReversible::apply_inverse` merges
//! the patch back into the current row, so a concurrent rename or
//! description edit is preserved across the undo.
//!
//! Why this verb finally lands: closes the cascade-on-delete
//! footgun documented in
//! `rubix/docs/sessions/undo/2026-05-28-team-crud-closeout.md`.
//! Before unassign existed, the only way to clear a team's
//! membership was to delete the whole team (the
//! refuse-if-members rejected option). Now operators have an
//! escape valve, and a future change to `team.delete`'s cascade
//! posture is debatable on its own merits rather than forced.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.unassign`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamUnassignRequest {
    /// Stable id of the target team.
    pub team_id: String,
    /// Stable id of the user to remove.
    pub user_id: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamUnassignResponse {
    /// Outcome — `rubix.team.unassigned`. The same code is
    /// returned for both the first unassignment and a no-op
    /// re-unassignment; callers distinguish via
    /// `already_not_member`.
    pub summary: Diagnostic,
    /// Echoed team id.
    pub team_id: String,
    /// Echoed user id.
    pub user_id: String,
    /// `true` when the user was not a member on entry — the verb
    /// is idempotent and reports the prior state.
    pub already_not_member: bool,
    /// Epoch milliseconds (UTC) at which the unassignment took
    /// effect. When `already_not_member` is `true`, this is just
    /// the call timestamp (there is no prior unassigned-at).
    pub unassigned_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "teams.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Remove an existing user from an existing team.",
    when_to_use: concat!(
        "Use when an operator says \"remove Ada from the Ops team\", ",
        "when a flow is offboarding a user, or when emptying a team ",
        "before deleting it."
    ),
    when_not_to_use: concat!(
        "Do not use to delete the whole team (that is rubix.team.delete; ",
        "it cascades). Do not use to add a member (that is ",
        "rubix.team.assign). Do not use to disable a user (that is ",
        "rubix.user.disable)."
    ),
    example: concat!(
        "Input:  { \"team_id\": \"t-1\", \"user_id\": \"u-1\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.unassigned\" }, ",
        "\"team_id\": \"t-1\", \"user_id\": \"u-1\", ",
        "\"already_not_member\": false, \"unassigned_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.team.assign",
            wins_when: "the caller wants to ADD a member, not remove one.",
        },
        SiblingTool {
            id: "rubix.team.delete",
            wins_when: "the caller wants to remove the whole team (cascades through members).",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE an unassignment they just performed.",
        },
    ],
};
