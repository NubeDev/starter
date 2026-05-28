//! `AuditPolicyStore` async trait + value type.
//!
//! Mirrors [`crate::dashboard::store`] in shape: the trait is the
//! seam every `rubix.audit.policy.*` verb body dispatches through;
//! the production impl is [`rubix-store-postgres::audit::PgAuditPolicyStore`]
//! and the in-memory test fake is `rubix-tools::audit::store::InMemoryAuditPolicyStore`.
//!
//! Contract:
//!
//! - **Row shape is canonical.** `(resource_kind, max_age_days, updated_at_ms)`
//!   \u{2014} the row is small enough that the audit Reversible uses a
//!   full snapshot (not a patch) for `before`/`after`.
//! - **`max_age_days = None` pins the kind to "keep forever".**
//!   `Some(n)` applies a finite retention curve in days.
//! - **`updated_at_ms` is store-canonical.** Implementations stamp
//!   it at write time; callers' values are advisory. `put()`
//!   bypasses this (it's the undo path and must restore the
//!   byte-exact prior timestamp).
//! - **Idempotent upserts.** Same kind + same `max_age_days` is a
//!   no-op: implementations MUST return `(Some(prior), prior)`
//!   without touching `updated_at`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::error::Result;

/// Resource-kind discriminator for audit-policy rows.
pub const AUDIT_POLICY_KIND: &str = "audit_policy";

/// One row in `changelog_kind_policy`.
///
/// `max_age_days = None` pins the kind to "keep forever".
/// `Some(n)` applies a finite retention curve in days.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditPolicyRow {
    /// Resource kind the policy applies to.
    pub resource_kind: String,
    /// Retention curve in days. `None` = pinned to forever.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_age_days: Option<i32>,
    /// Epoch milliseconds (UTC) at which the row was last
    /// upserted. Stored on the row so undo restores the
    /// byte-exact prior timestamp \u{2014} not a fresh `NOW()`.
    pub updated_at_ms: i64,
}

/// Persistence surface the audit-policy verbs target.
#[async_trait]
pub trait AuditPolicyStore: Send + Sync {
    /// List every row ordered by `resource_kind` ascending.
    /// Stable order is part of the contract: callers (the
    /// `list` verb) surface rows directly to operators.
    async fn list(&self) -> Result<Vec<AuditPolicyRow>>;
    /// Fetch a single row by kind. `None` when no row exists
    /// (the kind is implicitly unbounded).
    async fn get(&self, resource_kind: &str) -> Result<Option<AuditPolicyRow>>;
    /// Upsert. Returns `(prior_row, new_row)` \u{2014} `prior_row`
    /// is `None` when the upsert is an insert. Implementations
    /// MUST stamp `updated_at_ms` on the new row at write time
    /// (the caller's `updated_at_ms` is advisory; the store is
    /// canonical). On a no-op (same kind + same `max_age_days`)
    /// implementations MUST return `(Some(prior), prior)`
    /// without touching `updated_at` \u{2014} the verb relies on
    /// this to detect idempotency.
    async fn upsert(
        &self,
        resource_kind: &str,
        max_age_days: Option<i32>,
    ) -> Result<(Option<AuditPolicyRow>, AuditPolicyRow)>;
    /// Restore a row to the supplied snapshot. Used by the
    /// audit Reversible's `apply_inverse` to undo an upsert.
    /// Bypasses idempotency \u{2014} the snapshot must land
    /// verbatim, including its `updated_at_ms`.
    async fn put(&self, row: AuditPolicyRow) -> Result<()>;
    /// Hard-delete a row by kind. Idempotent on missing rows
    /// (undo of an insert deletes; if the row is already gone
    /// the undo still succeeds). Used by `apply_inverse` to
    /// undo `Op::Create`.
    async fn delete(&self, resource_kind: &str) -> Result<()>;
}
