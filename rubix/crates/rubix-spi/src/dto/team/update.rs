//! `rubix.team.update` — request/response DTOs and tool descriptor.
//!
//! Mutates the human-facing fields of an existing team row —
//! `name` and/or `description`. The id is immutable (renaming a
//! `team_id` would invalidate every audit row referencing the
//! team). Membership is mutated through `rubix.team.assign`, not
//! through update — update is for name/description only.
//!
//! Idempotency: when every requested field already matches the
//! stored row, the verb returns `rubix.team.unchanged` and
//! [`ReversibleTool::change_for`] returns `None`. Same posture as
//! [`crate::tenant::update`] / [`crate::user::role_set`].
//!
//! Snapshot shape: **patch** (matching
//! [`crate::team::store::TeamReversible`]'s payload contract).
//! Only the fields the verb actually flipped land in the
//! `before`/`after` patches. This is why an update verb and an
//! assign verb can run concurrently without clobbering each other
//! on undo — the patches address disjoint fields.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.team.update`.
///
/// `team_id` is required. `name` and `description` are both
/// optional; at least one must be `Some(...)`. Field-level `None`
/// means "leave alone"; an explicit empty `name` is rejected.
/// Setting `description` to an empty string is allowed and
/// stored verbatim (operators occasionally use "" to clear a
/// description; native `null` clearing would require
/// `Option<Option<String>>` which serde does not distinguish
/// from a missing field by default — keeping the type flat).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TeamUpdateRequest {
    /// Id of the team row to mutate. Required.
    pub team_id: String,
    /// New name. When `Some`, must be non-empty and trimmed.
    /// `None` leaves the name unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New description. `None` leaves it unchanged; `Some("")`
    /// stores an empty description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Tool reply.
///
/// Echoes both the prior and new values of every field the verb
/// can mutate, so [`ReversibleTool::change_for`] reconstructs the
/// `before`/`after` patches from the response alone (no follow-up
/// store read — proposal §3.1 fix).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamUpdateResponse {
    /// Outcome — `rubix.team.updated` or `rubix.team.unchanged`.
    pub summary: Diagnostic,
    /// Echoed id.
    pub team_id: String,
    /// Name as of pre-update.
    pub prior_name: String,
    /// Name as of post-update (== `prior_name` when not requested
    /// or matched).
    pub new_name: String,
    /// Description as of pre-update.
    pub prior_description: Option<String>,
    /// Description as of post-update (== `prior_description` when
    /// not requested or matched).
    pub new_description: Option<String>,
    /// `true` when every requested field already matched.
    pub was_unchanged: bool,
    /// Epoch milliseconds (UTC) at which the update took effect.
    pub updated_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "teams.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Rename a team or update its description (id stays the same).",
    when_to_use: concat!(
        "Use when the operator says \"rename team t-ops to Operations\" ",
        "or \"set team t-ops description to ...\". At least one of name ",
        "/ description must be supplied."
    ),
    when_not_to_use: concat!(
        "Do not use to change a team id — ids are immutable. Do not ",
        "use to add or remove members (that is rubix.team.assign). Do ",
        "not use to provision a new team (that is rubix.team.create)."
    ),
    example: concat!(
        "Input:  { \"team_id\": \"t-ops\", \"name\": \"Operations\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.team.updated\", ",
        "\"params\": { \"team\": \"t-ops\", \"prior_name\": \"Ops\", ",
        "\"new_name\": \"Operations\" } }, \"team_id\": \"t-ops\", ",
        "\"prior_name\": \"Ops\", \"new_name\": \"Operations\", ",
        "\"prior_description\": null, \"new_description\": null, ",
        "\"was_unchanged\": false, \"updated_at_ms\": ... }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a team update they just performed.",
        },
        SiblingTool {
            id: "rubix.team.assign",
            wins_when: "the caller wants to add a user to the team, not rename it.",
        },
    ],
};
