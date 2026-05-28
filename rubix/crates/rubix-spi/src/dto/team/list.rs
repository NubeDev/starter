//! `rubix.team.list` \u{2014} request/response DTOs and tool descriptor.
//!
//! Read-only verb. Surfaces every team row, including the
//! membership count, so operators (and the agent) can:
//!
//! - confirm a team exists before `rubix.team.assign` /
//!   `rubix.team.unassign`,
//! - resolve the `rubix.user.in_teams` diagnostic the
//!   `rubix.user.delete` cascade emits (the diagnostic names
//!   up to 10 teams; this verb is how the operator enumerates
//!   the full set when the cap bites),
//! - power admin-UI team pickers without round-tripping the
//!   raw store.
//!
//! Echoes `member_count` rather than the full member list to
//! keep the response payload bounded \u{2014} a team with
//! thousands of members would balloon a single response. The
//! per-team membership detail is intentionally out of scope for
//! v1; if a future verb needs it, it should be
//! `rubix.team.get` with its own pagination, not a list
//! response that's unbounded by definition.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.list`. Empty for v1; future
/// filters (e.g. `member: Option<String>`) should be additive
/// and default-skipped so existing callers keep working.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TeamListRequest {}

/// One team as returned by `rubix.team.list`.
///
/// Carries `member_count` instead of the full membership map.
/// See the module doc for the bounding rationale.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamListItem {
    /// Stable id.
    pub team_id: String,
    /// Human-facing name. UNIQUE across the store.
    pub name: String,
    /// Optional description (only emitted when set).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Number of members currently assigned to this team.
    pub member_count: usize,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamListResponse {
    /// Outcome (`rubix.team.listed`).
    pub summary: Diagnostic,
    /// Total row count surfaced.
    pub count: usize,
    /// Rows sorted by `name` ascending for stable rendering.
    pub teams: Vec<TeamListItem>,
}

/// `starter-authz` permission string the caller must hold.
///
/// `teams.read` mirrors the pattern of `tenants.read` /
/// `users.read` (siblings keep symmetric read scopes); the
/// `teams.write` scope guards the mutating verbs. Read
/// permission is broader-by-design \u{2014} every operator
/// who can assign needs to see the destination set.
pub const REQUIRED_PERMISSION: &str = "teams.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List rubix teams with id, name, optional description, and current member count.",
    when_to_use: concat!(
        "Use to enumerate teams before assigning/unassigning a user, ",
        "to resolve a `rubix.user.in_teams` diagnostic surfaced by ",
        "`rubix.user.delete` (the diagnostic caps at 10 team names), ",
        "or to power an admin team picker. Membership counts are ",
        "included so the caller can spot empty / oversized teams."
    ),
    when_not_to_use: concat!(
        "Do not use to inspect a single team's full member list \u{2014} ",
        "the response carries `member_count`, not the member ids. ",
        "Do not use to create or modify a team; team writes are ",
        "`rubix.team.{create,update,delete,assign,unassign}`."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.listed\", ",
        "\"params\": { \"count\": 2 } }, \"count\": 2, ",
        "\"teams\": [ { \"team_id\": \"t-ops\", \"name\": \"Ops\", ",
        "\"member_count\": 3 }, { \"team_id\": \"t-sre\", \"name\": ",
        "\"SRE\", \"description\": \"On-call\", \"member_count\": 5 } ] }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.user.list",
            wins_when:
                "the caller wants USERS, not teams. user.list scopes the question one level down.",
        },
        SiblingTool {
            id: "rubix.team.unassign",
            wins_when:
                "the caller already knows the team id and wants to remove a member from it.",
        },
    ],
};
