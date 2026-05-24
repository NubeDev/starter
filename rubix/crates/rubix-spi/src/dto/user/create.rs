//! `rubix.user.create` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles). See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! for the verb contract and the snapshot shape used by the
//! `Reversible` impl.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.create`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserCreateRequest {
    /// Login email for the new user.
    pub email: String,
    /// Role to assign. Accepts `reader`, `writer`, `admin`.
    pub role: String,
    /// Optional argon2id PHC hash; `None` means a third-party
    /// sign-in path will set the password later.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password_hash: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserCreateResponse {
    /// Outcome (`rubix.user.created`).
    pub summary: Diagnostic,
    /// Stable id of the new row.
    pub user_id: String,
    /// Email the row was created with (echoes the request).
    pub email: String,
    /// Role string the row was created with.
    pub role: String,
    /// Epoch milliseconds (UTC) at which the row was created.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "users.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Create a new user with a chosen email and role.",
    when_to_use: concat!(
        "Use when an operator says \"add a user\", \"invite a teammate\", ",
        "or when a flow needs to provision an account before assigning ",
        "it to a team."
    ),
    when_not_to_use: concat!(
        "Do not use to re-enable a previously disabled user (call ",
        "rubix.undo.last against the disable). Do not use to change ",
        "an existing user's role."
    ),
    example: concat!(
        "Input:  { \"email\": \"ada@example.com\", \"role\": \"admin\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.created\", ",
        "\"params\": { \"email\": \"ada@example.com\", \"role\": \"admin\" } }, ",
        "\"user_id\": \"u-...\", \"email\": \"ada@example.com\", ",
        "\"role\": \"admin\", \"created_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.user.disable",
            wins_when: "the caller wants to provision a NEW user, not gate an existing one.",
        },
        SiblingTool {
            id: "rubix.team.assign",
            wins_when: "the user does not yet exist; assign needs an existing user_id.",
        },
    ],
};
