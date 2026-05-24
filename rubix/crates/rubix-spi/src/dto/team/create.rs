//! `rubix.team.create` — request/response DTOs and tool descriptor.
//!
//! See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.create`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamCreateRequest {
    /// Human-facing team name. Trimmed; must be non-empty.
    pub name: String,
    /// Optional one-line description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamCreateResponse {
    /// Outcome (`rubix.team.created`).
    pub summary: Diagnostic,
    /// Stable id of the new team row.
    pub team_id: String,
    /// Echoed name.
    pub name: String,
    /// Epoch milliseconds (UTC) at which the row was created.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "teams.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Create a new team that users can be assigned to.",
    when_to_use: concat!(
        "Use when an operator says \"add a team called X\" or when a ",
        "flow is provisioning a new group before assigning members."
    ),
    when_not_to_use: concat!(
        "Do not use to add a user to an existing team (that is ",
        "rubix.team.assign). Do not use to rename an existing team."
    ),
    example: concat!(
        "Input:  { \"name\": \"Ops\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.created\", ",
        "\"params\": { \"name\": \"Ops\" } }, ",
        "\"team_id\": \"t-...\", \"name\": \"Ops\", ",
        "\"created_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.team.assign",
            wins_when: "the team already exists and the caller wants to add a member.",
        },
    ],
};
