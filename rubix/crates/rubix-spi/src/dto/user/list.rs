//! `rubix.user.list` — request/response DTOs and tool descriptor.
//!
//! Read-only verb. Surfaces every user row the caller has visibility
//! over; the response carries a `Diagnostic` (`rubix.user.listed`)
//! plus a structured `users` array. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! for the verb contract.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.user.list`.
///
/// Empty for v1 — listing is unfiltered. Future revisions will grow
/// optional `tenant_id` / `role` / `disabled` filters; those land
/// when the PG-backed `UserAdminStore` does.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct UserListRequest {}

/// One user as returned by `rubix.user.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListItem {
    /// Stable id.
    pub user_id: String,
    /// Login email.
    pub email: String,
    /// Role string (`reader` / `writer` / `admin`).
    pub role: String,
    /// `Some(epoch_ms)` when the row is disabled, `None` when
    /// enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    /// Outcome (`rubix.user.listed`).
    pub summary: Diagnostic,
    /// Total row count surfaced.
    pub count: usize,
    /// Rows sorted by `email` ascending for stable rendering.
    pub users: Vec<UserListItem>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "users.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List rubix users visible to the caller, with id, email, role, and disabled state.",
    when_to_use: concat!(
        "Use to verify a user-admin write *after* it ran (a stale ",
        "list before the write gives no useful signal), or when the ",
        "operator asks \"who do we have?\"."
    ),
    when_not_to_use: concat!(
        "Do not use to look up a single user by id or email when the ",
        "caller already knows it — that is rubix.user.get (not yet ",
        "wired). Do not use as the pre-check before a mutation; ",
        "verify after."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.user.listed\", ",
        "\"params\": { \"count\": 3 } }, \"count\": 3, ",
        "\"users\": [ { \"user_id\": \"u-...\", \"email\": ",
        "\"ada@example.com\", \"role\": \"admin\" }, ... ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.tenant.list",
        wins_when:
            "the caller wants USERS, not tenants. tenant.list scopes the question one level up.",
    }],
};
