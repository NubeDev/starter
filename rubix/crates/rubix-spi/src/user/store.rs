//! `UserAdminStore` async trait + value type.
//!
//! Contract:
//!
//! - **Row shape is canonical.**
//!   `(user_id, email, role, disabled_at_ms, prefs_json, tenant_id)`.
//!   Snapshot-shape Reversible \u{2014} `before`/`after` carry the
//!   full row.
//! - **`create` enforces uniqueness on `email`.** Operator-visible
//!   key. Returns [`starter_spi::error::Error::Conflict`].
//! - **All mutating verbs return `(prior, new)` and are
//!   idempotent.** When the requested state already holds,
//!   implementations MUST return `(prior, prior)` without
//!   touching the row \u{2014} the verb relies on this to detect
//!   no-ops (\u{00A7}3.4 redo-stack-clear contract).
//! - **`put` bypasses idempotency.** Undo path; snapshot lands
//!   verbatim.
//! - **`delete` is idempotent on missing rows.** Symmetric with
//!   undo of a create (the row may already be gone).
//! - **Tenant FK is verb-level**, not store-level: the store does
//!   NOT validate `tenant_id` resolves; the
//!   `rubix.user.tenant.assign` verb does (the rubix-side
//!   `TenantStore` is a sibling module). The Pg impl carries a
//!   real `FOREIGN KEY ... ON DELETE RESTRICT` for defense in
//!   depth.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use starter_spi::error::Result;

/// Resource-kind discriminator. Matches `ResourceRef::kind` on
/// every recorded `Change` for a user row.
pub const USER_KIND: &str = "user";

/// One user row as persisted by the rubix user-admin verbs.
///
/// Snapshot shape `UserReversible` reads/writes via
/// `Change::before` / `Change::after`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserRow {
    /// Stable id (assigned at create time).
    pub user_id: String,
    /// Login email.
    pub email: String,
    /// Role string (`reader` / `writer` / `admin`).
    pub role: String,
    /// `Some(epoch_ms)` when the user is disabled, `None` when
    /// enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_at_ms: Option<i64>,
    /// Free-form per-user preferences. `None` means "no prefs
    /// row" \u{2014} semantically different from
    /// `Some(Value::Null)` which means "prefs explicitly cleared".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefs_json: Option<Value>,
    /// Tenant assignment. `None` = unassigned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

/// Persistence surface the user-admin verbs target.
#[async_trait]
pub trait UserAdminStore: Send + Sync {
    /// Insert a new user. Returns the row that landed. Returns
    /// `Error::Conflict` when `email` already exists.
    async fn create(&self, row: UserRow) -> Result<UserRow>;
    /// Mark a user as disabled and return `(prior_row, new_row)`.
    /// When the row is already disabled, both halves are equal
    /// and `new.disabled_at_ms` is the prior (unchanged)
    /// timestamp \u{2014} the verb reports
    /// `was_already_disabled = true`.
    async fn disable(&self, user_id: &str, now_ms: i64) -> Result<(UserRow, UserRow)>;
    /// Clear the `disabled_at_ms` marker and return
    /// `(prior_row, new_row)`. When the row was already enabled,
    /// both halves are equal and the verb reports
    /// `was_already_enabled = true`.
    async fn enable(&self, user_id: &str) -> Result<(UserRow, UserRow)>;
    /// Set the role on a user and return `(prior_row, new_row)`.
    /// When the row already carries `role`, both halves are equal.
    async fn set_role(&self, user_id: &str, role: &str) -> Result<(UserRow, UserRow)>;
    /// Replace the prefs blob on a user and return `(prior, new)`.
    /// When the stored blob is byte-equal to `prefs`, both halves
    /// are equal.
    async fn set_prefs(&self, user_id: &str, prefs: Value) -> Result<(UserRow, UserRow)>;
    /// Assign (or unassign) the tenant on a user row and return
    /// `(prior, new)`. `tenant_id = Some(id)` assigns, `None`
    /// unassigns. When the row already carries the requested
    /// value, both halves are equal. The store does NOT validate
    /// that `tenant_id` resolves in the sibling `TenantStore`;
    /// the verb does that check before calling.
    async fn set_tenant(
        &self,
        user_id: &str,
        tenant_id: Option<String>,
    ) -> Result<(UserRow, UserRow)>;
    /// Fetch by user_id.
    async fn get(&self, user_id: &str) -> Result<Option<UserRow>>;
    /// Fetch by email.
    async fn find_by_email(&self, email: &str) -> Result<Option<UserRow>>;
    /// List all rows. Order is unspecified \u{2014} callers sort if
    /// they need stability.
    async fn list(&self) -> Result<Vec<UserRow>>;
    /// Restore (or insert) a row to the supplied snapshot. Used
    /// by `UserReversible::apply_inverse` to walk a `Change`
    /// backwards. Bypasses idempotency.
    async fn put(&self, row: UserRow) -> Result<()>;
    /// Hard-delete a row by id. Idempotent on missing rows.
    async fn delete(&self, user_id: &str) -> Result<()>;
}
