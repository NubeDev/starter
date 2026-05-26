//! `rubix.tenant.list` — request/response DTOs and tool descriptor.
//!
//! Read-only verb. Surfaces every tenant row visible to the caller
//! so the user-admin agent can confirm tenant context before mutating
//! a user. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.tenant.list`. Empty for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TenantListRequest {}

/// One tenant as returned by `rubix.tenant.list`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantListItem {
    /// Stable id.
    pub tenant_id: String,
    /// Human-facing name.
    pub name: String,
    /// IETF locale tag (e.g. `en`, `es`).
    pub locale: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantListResponse {
    /// Outcome (`rubix.tenant.listed`).
    pub summary: Diagnostic,
    /// Total row count surfaced.
    pub count: usize,
    /// Rows sorted by `name` ascending for stable rendering.
    pub tenants: Vec<TenantListItem>,
}

/// `starter-authz` permission string the caller must hold.
pub const REQUIRED_PERMISSION: &str = "tenants.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List rubix tenants visible to the caller, with id, name, and locale.",
    when_to_use: concat!(
        "Use before mutating a user to confirm which tenant the row ",
        "belongs to. Cross-tenant writes are a common source of ",
        "\"I disabled the wrong account\" incidents."
    ),
    when_not_to_use: concat!(
        "Do not use to enumerate USERS within a tenant — that is ",
        "rubix.user.list. Do not use to create or modify a tenant; ",
        "tenant writes are out of scope for this phase."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.tenant.listed\", ",
        "\"params\": { \"count\": 2 } }, \"count\": 2, ",
        "\"tenants\": [ { \"tenant_id\": \"t-acme\", \"name\": ",
        "\"Acme\", \"locale\": \"en\" }, ... ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.user.list",
        wins_when:
            "the caller wants USERS, not tenants. user.list scopes the question one level down.",
    }],
};
