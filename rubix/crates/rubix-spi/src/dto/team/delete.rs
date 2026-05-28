//! `rubix.team.delete` — request/response DTOs and tool descriptor.
//!
//! Hard-deletes a team row, including its membership map.
//!
//! Cascade decision: **allow with membership disclosure**. Unlike
//! `rubix.tenant.delete` (which refuses when users are assigned),
//! team membership is stored *inside* the team row as a
//! `BTreeMap<user_id, assigned_at_ms>`, not as an external FK on
//! the `UserRow`. Deleting a team therefore creates no orphaned
//! references — the user rows are unaffected, only the in-team
//! membership records vanish. The verb echoes the prior member
//! count in the diagnostic so the operator sees what they
//! deleted, and the snapshot in the audit row carries the full
//! membership map so undo restores the team byte-exact (members
//! included).
//!
//! Alternative considered and rejected: "refuse if members > 0".
//! Would deadlock the operator because there is no `team.unassign`
//! verb today — once a user lands in a team, the only way to
//! remove them is to delete the team (or wait for `team.unassign`
//! to be implemented). A refuse-shaped delete with no unassign
//! verb is a footgun.
//!
//! Snapshot shape: `Op::Delete`, `before` = full prior
//! [`crate::team::store::TeamRow`] (including the members map),
//! `after = None`. [`crate::team::store::TeamReversible`]
//! `apply_inverse` matches `Op::Delete` by calling `store.put` on
//! the full snapshot — restoration is byte-exact.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.delete`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamDeleteRequest {
    /// Id of the team row to delete.
    pub team_id: String,
}

/// Tool reply.
///
/// Echoes the full identity of the deleted row so
/// [`ReversibleTool::change_for`] reconstructs the snapshot
/// without a follow-up store read.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamDeleteResponse {
    /// Outcome (`rubix.team.deleted`). Diagnostic params include
    /// `team` (id), `name`, `members` (count), and `at`.
    pub summary: Diagnostic,
    /// Echoed id.
    pub team_id: String,
    /// Name of the deleted team.
    pub name: String,
    /// Description of the deleted team.
    pub description: Option<String>,
    /// `user_id -> assigned_at_ms` map at the time of deletion.
    /// `BTreeMap` ordering preserved on the wire so undo replays
    /// the exact bytes.
    pub members: std::collections::BTreeMap<String, i64>,
    /// Number of members that were assigned at the time of
    /// deletion. Pulled into a separate field (not just
    /// `members.len()`) so callers can branch on it without
    /// allocating.
    pub member_count: usize,
    /// Epoch milliseconds (UTC) at which the delete took effect.
    pub deleted_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same `teams.write` permission as create / update — all three
/// are team-lifecycle verbs.
pub const REQUIRED_PERMISSION: &str = "teams.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Delete a team (cascades through its membership map).",
    when_to_use: concat!(
        "Use when offboarding a team, when an operator says \"remove ",
        "team t-ops\", or when cleaning up a staging team. Member ",
        "user rows are NOT affected — only the team itself and its ",
        "membership records vanish."
    ),
    when_not_to_use: concat!(
        "Do not use to remove a single user from a team — team ",
        "membership currently has no unassign verb; deleting the ",
        "whole team is the only way to clear members today. Do not ",
        "use to rename a team (that is rubix.team.update)."
    ),
    example: concat!(
        "Input:  { \"team_id\": \"t-ops\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.deleted\", ",
        "\"params\": { \"name\": \"Ops\", \"members\": 3 } }, ",
        "\"team_id\": \"t-ops\", \"name\": \"Ops\", \"description\": null, ",
        "\"members\": { \"u-1\": 1764892800000 }, \"member_count\": 3, ",
        "\"deleted_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a team delete (restores members too).",
        },
        SiblingTool {
            id: "rubix.team.update",
            wins_when: "the caller wants to rename or re-describe the team, not remove it.",
        },
    ],
};
