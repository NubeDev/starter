//! `TenantStore` async trait + value type.
//!
//! Contract:
//!
//! - **Row shape is canonical.** `(tenant_id, name, locale)`
//!   \u{2014} no store-managed timestamps, so the \u{00A7}3.1
//!   echo rule reduces to "echo the three fields".
//! - **`create` enforces uniqueness on both `tenant_id` AND
//!   `name`.** Both are operator-visible keys. Returns
//!   [`starter_spi::error::Error::Conflict`] on collision.
//! - **`put` bypasses uniqueness.** It is the undo path; the
//!   snapshot must land verbatim even if a transient concurrent
//!   write briefly held the id/name.
//! - **`delete` returns `NotFound` when the id does not resolve.**
//!   The store does NOT enforce the "no assigned users" check
//!   \u{2014} that is a verb-level concern (`crate::user`
//!   sibling lives in a different store).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::error::Result;

/// Resource-kind discriminator for tenant rows.
pub const TENANT_KIND: &str = "tenant";

/// One tenant row as surfaced by `rubix.tenant.list` and
/// mutated by `rubix.tenant.{create,update,delete}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantRow {
    /// Stable id.
    pub tenant_id: String,
    /// Human-facing name.
    pub name: String,
    /// IETF locale tag (e.g. `en`, `es`). Returned so the caller
    /// can localise per-tenant follow-up prompts.
    pub locale: String,
}

/// Persistence surface the tenant verbs target.
#[async_trait]
pub trait TenantStore: Send + Sync {
    /// List all tenant rows. Order is unspecified \u{2014} callers
    /// sort if they need stability.
    async fn list(&self) -> Result<Vec<TenantRow>>;
    /// Fetch a single tenant row by id. Returns `None` when the
    /// id does not resolve. The default impl walks `list()` so
    /// simple implementors keep working; production impls override
    /// with an indexed lookup.
    async fn get(&self, tenant_id: &str) -> Result<Option<TenantRow>> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .find(|r| r.tenant_id == tenant_id))
    }
    /// Insert a new tenant. Returns the row that landed. Returns
    /// `Error::Conflict` when either `tenant_id` or `name`
    /// already exists.
    async fn create(&self, row: TenantRow) -> Result<TenantRow>;
    /// Restore (or replace) a row to the supplied snapshot. Used
    /// by the tenant Reversible's `apply_inverse` to walk a
    /// `Change` backwards. Bypasses uniqueness.
    async fn put(&self, row: TenantRow) -> Result<()>;
    /// Hard-delete a row by id. Returns `Error::NotFound` when
    /// the id does not resolve.
    async fn delete(&self, tenant_id: &str) -> Result<()>;
}
