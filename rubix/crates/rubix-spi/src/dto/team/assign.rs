//! `rubix.team.assign` — request/response DTOs and tool descriptor.
//!
//! See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.assign`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamAssignRequest {
    /// Stable id of the target team.
    pub team_id: String,
    /// Stable id of the user to add.
    pub user_id: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamAssignResponse {
    /// Outcome — `rubix.team.assigned`. The same code is returned
    /// for both the first assignment and a no-op re-assignment;
    /// callers distinguish via `already_member`.
    pub summary: Diagnostic,
    /// Echoed team id.
    pub team_id: String,
    /// Echoed user id.
    pub user_id: String,
    /// `true` when the user was already a member on entry — the
    /// verb is idempotent and reports the prior state.
    pub already_member: bool,
    /// Epoch milliseconds (UTC) at which the membership took
    /// effect. When `already_member` is `true`, this is the prior
    /// assigned-at timestamp.
    pub assigned_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "teams.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Add an existing user to an existing team.",
    when_to_use: concat!(
        "Use when an operator says \"add Ada to the Ops team\" or when ",
        "a flow is finishing user onboarding."
    ),
    when_not_to_use: concat!(
        "Do not use to create a team (call rubix.team.create first). ",
        "Do not use to create a user (call rubix.user.create first). ",
        "Do not use to remove a member (that is rubix.team.unassign)."
    ),
    example: concat!(
        "Input:  { \"team_id\": \"t-1\", \"user_id\": \"u-1\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.assigned\" }, ",
        "\"team_id\": \"t-1\", \"user_id\": \"u-1\", ",
        "\"already_member\": false, \"assigned_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.team.create",
            wins_when: "the team does not yet exist; assign needs an existing team_id.",
        },
        SiblingTool {
            id: "rubix.user.create",
            wins_when: "the user does not yet exist; assign needs an existing user_id.",
        },
    ],
};
