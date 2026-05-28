//! `rubix.tenant.create` — request/response DTOs and tool descriptor.
//!
//! Provisions a new tenant. The verb is NOT idempotent: a second
//! call with the same id or name returns `Error::Conflict`, not a
//! silent no-op. Tenants are operator-facing identity boundaries,
//! and making create silent-idempotent would let two operators
//! think they each "own" a tenant they share — a worse failure
//! mode than the conflict surface.
//!
//! Snapshot shape: `Op::Create`, `after` = the full new
//! [`crate::tenant::store::TenantRow`], `before = None`. The
//! [`TenantReversible::apply_inverse`] path deletes the row,
//! same as every other `Op::Create`-shaped Reversible.
//!
//! Audit posture: tenant lifecycle is security-relevant (it
//! controls per-tenant data visibility). The seeded
//! `changelog_kind_policy` row added in the rubix-side
//! `changelog_policy/0002_*.sql` migration pins the `tenant` kind
//! to `max_age_days = NULL` (keep forever) — same posture as
//! `user` and `team`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.tenant.create`.
///
/// `tenant_id` is optional. When `None`, the verb generates a
/// `t-<uuid>` id (mirrors `rubix.user.create`'s `u-<uuid>`
/// posture). Providing an explicit id is supported for migration
/// flows that need stable cross-environment identifiers.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TenantCreateRequest {
    /// Optional explicit tenant id. Must be unique. When `None`,
    /// the verb generates a `t-<uuid>` id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// Human-facing name. Must be unique; the user-admin agent
    /// uses name as the operator-facing handle in confirmation
    /// prompts, and ambiguous names there are a footgun.
    pub name: String,
    /// IETF locale tag (`en`, `es`, …). Returned to callers so
    /// per-tenant follow-up prompts localise correctly. When
    /// `None`, the verb defaults to `"en"`; an explicit empty
    /// string is rejected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantCreateResponse {
    /// Outcome — `rubix.tenant.created`.
    pub summary: Diagnostic,
    /// Stable id assigned (or echoed) for the new tenant.
    pub tenant_id: String,
    /// Human-facing name of the new tenant.
    pub name: String,
    /// Locale of the new tenant.
    pub locale: String,
    /// Epoch milliseconds (UTC) at which create took effect.
    pub created_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Tenant lifecycle is a *higher* privilege than per-tenant user
/// writes — creating a tenant carves out a new identity
/// boundary. A separate `tenants.write` permission rather than
/// reusing `users.write`.
pub const REQUIRED_PERMISSION: &str = "tenants.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Provision a new tenant (identity boundary for per-tenant data).",
    when_to_use: concat!(
        "Use when onboarding a new organisation, when the operator ",
        "says \"add tenant acme\", or when a flow needs a fresh ",
        "identity boundary for staging data."
    ),
    when_not_to_use: concat!(
        "Do not use to assign a user to an existing tenant (that is ",
        "rubix.user.tenant.assign). Do not use to rename an existing ",
        "tenant (that is rubix.tenant.update)."
    ),
    example: concat!(
        "Input:  { \"name\": \"Acme\", \"locale\": \"en\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.tenant.created\", ",
        "\"params\": { \"name\": \"Acme\" } }, \"tenant_id\": \"t-...\", ",
        "\"name\": \"Acme\", \"locale\": \"en\", \"created_at_ms\": ... }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a tenant create they just performed.",
        },
        SiblingTool {
            id: "rubix.tenant.list",
            wins_when: "the caller wants to DISCOVER existing tenant ids, not provision a new one.",
        },
    ],
};
