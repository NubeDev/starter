//! `rubix.audit.policy.list` — request/response DTOs and tool descriptor.
//!
//! Read-only inspection of the `changelog_kind_policy` table.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input — no filters today. List returns every row.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AuditPolicyListRequest {}

/// One audit-policy entry.
///
/// `max_age_days = None` means the kind is **pinned to forever**
/// (rows are never swept). A positive integer applies that
/// retention curve. The table does not store the "implicit
/// unbounded" baseline — kinds with no row are implicitly
/// unbounded and absent from this listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct AuditPolicyEntry {
    /// Resource kind the policy applies to (e.g. `"user"`,
    /// `"team"`, `"tenant"`, `"flow_def"`).
    pub resource_kind: String,
    /// Retention curve in days. `None` pins the kind to
    /// "keep forever". `Some(n)` deletes audit rows older
    /// than `n` days at the next sweep.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
    /// Epoch milliseconds (UTC) at which the policy row was
    /// last upserted.
    pub updated_at_ms: i64,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditPolicyListResponse {
    /// Human-readable summary — `rubix.audit.policy.listed`.
    pub summary: Diagnostic,
    /// Every policy row, ordered by `resource_kind` ascending
    /// so output is stable across calls.
    pub entries: Vec<AuditPolicyEntry>,
}

/// `starter-authz` permission string the caller must hold.
///
/// Audit policy is operator-only. Reads are gated by
/// `audit.policy.read` so the surface can be granted
/// independently of the write half.
pub const REQUIRED_PERMISSION: &str = "audit.policy.read";

/// Five-field descriptor.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "List the audit-retention policy for every resource kind.",
    when_to_use: concat!(
        "Use when an operator asks \"what is the audit retention curve?\" ",
        "or \"is the user kind pinned to forever?\". Returns every row in ",
        "changelog_kind_policy; kinds with no row are implicitly unbounded ",
        "and are absent from the listing."
    ),
    when_not_to_use: concat!(
        "Do not use to change the policy (that is rubix.audit.policy.set). ",
        "Do not use to fetch audit rows themselves (the audit listing belongs ",
        "to a separate UI surface, not to a tool)."
    ),
    example: concat!(
        "Input:  {}\n",
        "Output: { \"summary\": { \"code\": \"rubix.audit.policy.listed\", ",
        "\"params\": { \"count\": 2 } }, \"entries\": [ ",
        "{ \"resource_kind\": \"team\", \"updated_at_ms\": 1764800000000 }, ",
        "{ \"resource_kind\": \"user\", \"updated_at_ms\": 1764800000000 } ] }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.audit.policy.set",
        wins_when: "the caller wants to CHANGE the retention curve, not just inspect it.",
    }],
};
