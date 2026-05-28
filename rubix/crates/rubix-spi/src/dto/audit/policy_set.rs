//! `rubix.audit.policy.set` — request/response DTOs and tool descriptor.
//!
//! Upsert a per-kind retention policy into
//! `changelog_kind_policy`. Reversible via `AuditPolicyReversible`
//! (snapshot shape). Idempotent — calling set with the same
//! `(kind, max_age_days)` value the row already carries returns
//! the `rubix.audit.policy.unchanged` diagnostic and produces
//! no `ChangeDraft`.
//!
//! See [`crate::dto::audit`] for the policy model.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.audit.policy.set`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditPolicySetRequest {
    /// Resource kind to pin (e.g. `"user"`, `"team"`,
    /// `"flow_def"`). Required, must be non-empty after trim.
    pub resource_kind: String,
    /// Retention curve in days.
    ///
    /// - `None` — pin the kind to "keep forever" (no sweep).
    /// - `Some(n)` where `n > 0` — sweep deletes audit rows
    ///   older than `n` days.
    ///
    /// The verb rejects `Some(n)` where `n <= 0` (0 or negative
    /// would either delete every new row instantly or be
    /// nonsense — neither is a useful operator intent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditPolicySetResponse {
    /// Outcome — `rubix.audit.policy.set` /
    /// `rubix.audit.policy.pinned` / `rubix.audit.policy.unchanged`.
    pub summary: Diagnostic,
    /// Resource kind the policy applies to (echoed).
    pub resource_kind: String,
    /// The new retention curve (echoed). `None` when the kind
    /// is now pinned to forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
    /// The retention curve the row carried *prior* to this
    /// upsert. Tri-state:
    ///
    /// - field absent (`None` after deserialise) — there was
    ///   no row before this call (the kind was implicitly
    ///   unbounded).
    /// - `Some(value)` where `value.max_age_days = None` —
    ///   the row existed and was pinned to forever.
    /// - `Some(value)` where `value.max_age_days = Some(n)`
    ///   — the row existed with a finite curve.
    ///
    /// Echoed so `change_for` reconstructs the full prior
    /// snapshot byte-exact (\u{00a7}3.1 echo rule). Without it
    /// undo would conflate "no policy" with "policy = forever".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<AuditPolicyPriorSnapshot>,
    /// `true` when the upsert was a no-op (same kind + same
    /// `max_age_days`). The verb skips the audit row in that
    /// case so undo cannot silently revert an unrelated edit.
    pub was_unchanged: bool,
    /// Epoch milliseconds (UTC) at which the upsert landed.
    /// When `was_unchanged` is `true`, this is the prior
    /// `updated_at` (unchanged because the row wasn't touched).
    pub updated_at_ms: i64,
}

/// Snapshot of the policy row prior to the upsert. Used inside
/// the `prior` field of [`AuditPolicySetResponse`] so undo
/// reconstructs the byte-exact state \u{2014} including the
/// `updated_at_ms` timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AuditPolicyPriorSnapshot {
    /// Prior retention curve. `None` when the row was pinned
    /// to forever, `Some(n)` for a finite curve.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
    /// Prior `updated_at` timestamp (epoch ms, UTC). Required
    /// for byte-exact undo: without it the restored row would
    /// carry a fresh `NOW()` instead of the original.
    pub updated_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold.
///
/// Write half of the operator surface. Granting `audit.policy.write`
/// implies trust to shorten retention curves \u{2014} a shortened
/// curve causes the next sweep to delete audit rows. Audit logs
/// for the policy change itself are immutable in the same table
/// (under the `audit_policy` kind), so the destructive intent is
/// itself recorded.
pub const REQUIRED_PERMISSION: &str = "audit.policy.write";

/// Resource kind used by [`AuditPolicyReversible`].
///
/// Picked verbatim so it appears stable in `starter_changes` and
/// can itself be pinned in `changelog_kind_policy` if operators
/// want the policy-change audit trail to outlive the standard
/// retention curve.
pub const AUDIT_POLICY_KIND: &str = "audit_policy";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Set or change the audit-retention policy for a single resource kind.",
    when_to_use: concat!(
        "Use when an operator says \"pin user audit to forever\", ",
        "\"set flow_def audit to 90 days\", or \"loosen tenant retention\". ",
        "Idempotent \u{2014} a second call with the same value is a no-op."
    ),
    when_not_to_use: concat!(
        "Do not use to inspect the current policy (that is ",
        "rubix.audit.policy.list). Do not use to delete audit rows ",
        "directly \u{2014} the sweep applies the policy at its own cadence."
    ),
    example: concat!(
        "Input:  { \"resource_kind\": \"user\", \"max_age_days\": null }\n",
        "Output: { \"summary\": { \"code\": \"rubix.audit.policy.pinned\", ",
        "\"params\": { \"kind\": \"user\" } }, \"resource_kind\": \"user\", ",
        "\"was_unchanged\": false, \"updated_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.audit.policy.list",
            wins_when: "the caller wants to read the current policy without changing it.",
        },
        SiblingTool {
            id: "rubix.undo.last",
            wins_when: "the caller wants to reverse a policy.set they just performed.",
        },
    ],
};
