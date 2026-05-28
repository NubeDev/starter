//! `rubix.tenant.delete` — request/response DTOs and tool descriptor.
//!
//! Deletes an existing tenant. The verb is **not** silent-
//! idempotent — calling delete on a missing id returns
//! `Error::NotFound`. Deletes are operator-visible and the
//! "I thought I already deleted that" question is better answered
//! by an explicit NotFound than by a silent success.
//!
//! ## Cascade decision: refuse if users assigned
//!
//! The verb refuses to delete a tenant that has any user
//! assigned to it via `UserRow.tenant_id`. The operator must
//! `rubix.user.tenant.assign` those users elsewhere (or
//! `tenant_id: null` to unassign) before the delete succeeds.
//!
//! The alternatives considered:
//!
//! - **Cascade-unassign** — silently flip every assigned user to
//!   `tenant_id = None`. Rejected: a delete that touches N user
//!   rows produces N audit entries that may surprise the operator
//!   later. The cascade also fans out across actor boundaries —
//!   the operator deleting the tenant may not own every assigned
//!   user. Worst, it lets an operator delete-then-immediately-
//!   recreate to forcibly unassign users from a tenant they don't
//!   technically have write permission over.
//! - **Block at the FK** — same effective behaviour as "refuse"
//!   but with an opaque database error rather than a structured
//!   diagnostic. Rejected for operator-experience reasons.
//! - **Refuse with a structured diagnostic** (this verb's
//!   choice) — the operator sees `rubix.tenant.has_users` with
//!   the count of assignments blocking the delete, can run
//!   `rubix.user.list` filtered by tenant, and fixes the
//!   underlying state explicitly.
//!
//! Snapshot shape: `Op::Delete`, `before` = the full prior
//! [`crate::tenant::store::TenantRow`], `after = None`. The
//! [`TenantReversible::apply_inverse`] path re-puts the row.
//! Note: undo of a delete restores the tenant row but does NOT
//! re-assign any users — those assignments were unwound by
//! `rubix.user.tenant.assign` before the delete and live in
//! their own audit chain. The operator chains the undos in
//! reverse order to fully restore.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.tenant.delete`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct TenantDeleteRequest {
    /// Stable id of the tenant to delete.
    pub tenant_id: String,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TenantDeleteResponse {
    /// Outcome — `rubix.tenant.deleted`.
    pub summary: Diagnostic,
    /// Stable id of the row that was deleted (echoed for
    /// confirmation; mirrors `rubix.user.disable.user_id`).
    pub tenant_id: String,
    /// Name of the row that was deleted. Echoed for the same
    /// reason and for the `change_for` snapshot reconstruction.
    pub name: String,
    /// Locale of the row that was deleted. Echoed for the
    /// `change_for` snapshot reconstruction (full-row `before`
    /// posture, same as the user verbs after the §3.1 bug-class
    /// fix).
    pub locale: String,
    /// Epoch milliseconds (UTC) at which delete took effect.
    pub deleted_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Same `tenants.write` permission as create — tenant lifecycle
/// is a single high-privilege scope, not split into per-op
/// permissions today.
pub const REQUIRED_PERMISSION: &str = "tenants.write";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Delete an existing tenant; refuses if any users are still assigned to it.",
    when_to_use: concat!(
        "Use when offboarding an organisation, when the operator ",
        "says \"remove tenant acme\", or when cleaning up a staging ",
        "tenant. The operator must unassign all users first — the ",
        "verb returns rubix.tenant.has_users when assignments block ",
        "the delete."
    ),
    when_not_to_use: concat!(
        "Do not use to unassign a single user from a tenant (that is ",
        "rubix.user.tenant.assign with tenant_id: null). Do not use ",
        "to disable a tenant temporarily — tenants are hard-deleted; ",
        "a soft-disable verb is a separate proposal."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"t-acme\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.tenant.deleted\", ",
        "\"params\": { \"name\": \"Acme\" } }, \"tenant_id\": \"t-acme\", ",
        "\"name\": \"Acme\", \"locale\": \"en\", \"deleted_at_ms\": ... }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.user.tenant.assign",
            wins_when: "the caller needs to UNASSIGN users from this tenant before delete will succeed.",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to REVERSE a tenant delete they just performed (restores the row).",
        },
    ],
};
