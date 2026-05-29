//! In-memory backing store + [`Reversible`] glue for the tenant
//! verbs.
//!
//! The trait + row type live in [`rubix_spi::tenant`] so this
//! crate and `rubix-store-postgres` share the same contract
//! without depending on each other (SCOPE R5: tools and
//! store-postgres are siblings, both rooted in `rubix-spi`).
//! The production binary swaps in
//! `rubix_store_postgres::PgRubixTenantStore` without touching
//! the verb files. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! \u{00A7}"Snapshot shape" for the JSON layout.
//!
//! Note: a *separate* `TenantStore` exists in `starter-auth-users`
//! covering the auth-side tenant directory. This rubix-side
//! `TenantStore` is the verb-surface store; the two are
//! intentionally separate today.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
// Re-export the contract from `rubix-spi::tenant` so existing verb
// code (`use crate::tenant::store::{TenantRow, TenantStore, TENANT_KIND}`)
// keeps compiling after the trait/row moved out of this crate.
pub use rubix_spi::tenant::{TenantRow, TenantStore, TENANT_KIND};
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::{Change, ChangeTx, Op, Reversible};
use starter_spi::error::{Error, Result};

/// In-memory [`TenantStore`] for tests and the in-process smoke
/// session.
#[derive(Default, Clone)]
pub struct InMemoryTenantStore {
    rows: Arc<Mutex<HashMap<String, TenantRow>>>,
}

impl InMemoryTenantStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed with the supplied rows. Last-row-wins on duplicate id
    /// (used by the registry boot path to install the bundled
    /// `System` tenant; production should not seed duplicates).
    pub fn seeded(rows: Vec<TenantRow>) -> Self {
        let map = rows.into_iter().map(|r| (r.tenant_id.clone(), r)).collect();
        Self {
            rows: Arc::new(Mutex::new(map)),
        }
    }

    /// Append a row, bypassing uniqueness. Test / smoke helper \u{2014}
    /// production callers go through the [`TenantStore`] trait's
    /// `create`.
    pub fn insert(&self, row: TenantRow) {
        self.lock().insert(row.tenant_id.clone(), row);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, TenantRow>> {
        self.rows.lock().expect("TenantStore mutex poisoned")
    }
}

#[async_trait]
impl TenantStore for InMemoryTenantStore {
    async fn list(&self) -> Result<Vec<TenantRow>> {
        Ok(self.lock().values().cloned().collect())
    }
    async fn get(&self, tenant_id: &str) -> Result<Option<TenantRow>> {
        Ok(self.lock().get(tenant_id).cloned())
    }
    async fn create(&self, row: TenantRow) -> Result<TenantRow> {
        let mut guard = self.lock();
        if guard.contains_key(&row.tenant_id) {
            return Err(Error::Conflict {
                message: format!("tenant with id {} already exists", row.tenant_id),
            });
        }
        if guard.values().any(|r| r.name == row.name) {
            return Err(Error::Conflict {
                message: format!("tenant with name {} already exists", row.name),
            });
        }
        guard.insert(row.tenant_id.clone(), row.clone());
        Ok(row)
    }
    async fn put(&self, row: TenantRow) -> Result<()> {
        self.lock().insert(row.tenant_id.clone(), row);
        Ok(())
    }
    async fn delete(&self, tenant_id: &str) -> Result<()> {
        let mut guard = self.lock();
        if guard.remove(tenant_id).is_none() {
            return Err(Error::NotFound {
                what: format!("tenant:{tenant_id}"),
            });
        }
        Ok(())
    }
}

/// Single [`Reversible`] impl for the `"tenant"` kind. Registered
/// once per server build alongside the tenant verbs.
///
/// Payload shape: **snapshot** (see
/// [`starter_spi::changelog::Reversible`] choice matrix). The
/// tenant row is tiny (three string fields) and the lifecycle
/// includes create/delete \u{2014} `before` is canonical full state,
/// `before == {}` marks "did not exist". Same posture as
/// [`crate::user::store::UserReversible`].
///
/// FK posture (cascade-on-undo): the verb `rubix.tenant.delete`
/// refuses to delete a tenant that has users assigned to it; an
/// operator must unassign first. `apply_inverse` here however
/// does NOT re-check \u{2014} undo replays the snapshot through
/// `store.put` / `store.delete` faithfully. Per-actor redo-stack
/// semantics (proposal \u{00A7}3.4) keep this safe in the normal
/// case: a single actor's undo chain walks back in reverse
/// mutation order, so the user assignments are unwound before
/// the tenant create is undone. If a different actor inserts
/// user assignments between the create and the undo, the undo
/// may delete a tenant that has users assigned \u{2014} that's a
/// cross-actor concurrency boundary, the same shape as every
/// other Reversible in the codebase (see `DashboardReversible`
/// for the precedent).
pub struct TenantReversible {
    store: Arc<dyn TenantStore>,
}

impl TenantReversible {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TenantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Reversible for TenantReversible {
    fn kind(&self) -> &'static str {
        TENANT_KIND
    }

    async fn apply_inverse(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "TenantReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create => self.store.delete(id).await,
            Op::Update => {
                let row: TenantRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Delete => {
                let row: TenantRow = parse_row(ch.before.as_ref(), "before")?;
                self.store.put(row).await
            }
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("TenantReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn apply_forward(&self, ch: &Change) -> Result<()> {
        let id = ch.resource.id.as_deref().ok_or_else(|| Error::Invalid {
            message: "TenantReversible: Change::resource.id is None".to_owned(),
        })?;
        match ch.op {
            Op::Create | Op::Update => {
                let row: TenantRow = parse_row(ch.after.as_ref(), "after")?;
                self.store.put(row).await
            }
            Op::Delete => self.store.delete(id).await,
            Op::Custom(ref op) => Err(Error::Invalid {
                message: format!("TenantReversible: unsupported custom op {op}"),
            }),
        }
    }

    async fn clone_with(
        &self,
        _tx: &dyn ChangeTx,
        _src: &ResourceRef,
        _overrides: serde_json::Value,
    ) -> Result<Vec<ResourceRef>> {
        // Cloning tenants is intentionally out of scope \u{2014} a
        // duplicated tenant would silently bypass the
        // (id, name) uniqueness operators rely on.
        Err(Error::Invalid {
            message: "tenant kind does not support clone".to_owned(),
        })
    }
}

fn parse_row(payload: Option<&serde_json::Value>, field: &str) -> Result<TenantRow> {
    let v = payload.ok_or_else(|| Error::Invalid {
        message: format!("TenantReversible: Change::{field} is None"),
    })?;
    serde_json::from_value::<TenantRow>(v.clone()).map_err(|e| Error::Invalid {
        message: format!("TenantReversible: Change::{field} is not a TenantRow: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, name: &str) -> TenantRow {
        TenantRow {
            tenant_id: id.into(),
            name: name.into(),
            locale: "en".into(),
        }
    }

    #[tokio::test]
    async fn create_rejects_duplicate_id() {
        let store = InMemoryTenantStore::new();
        store.create(row("t-1", "Acme")).await.unwrap();
        let err = store.create(row("t-1", "Different")).await.unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn create_rejects_duplicate_name() {
        let store = InMemoryTenantStore::new();
        store.create(row("t-1", "Acme")).await.unwrap();
        let err = store.create(row("t-2", "Acme")).await.unwrap_err();
        assert!(matches!(err, Error::Conflict { .. }));
    }

    #[tokio::test]
    async fn delete_on_missing_id_returns_not_found() {
        let store = InMemoryTenantStore::new();
        let err = store.delete("t-ghost").await.unwrap_err();
        assert!(matches!(err, Error::NotFound { .. }));
    }

    #[tokio::test]
    async fn put_overwrites_existing_row() {
        let store = InMemoryTenantStore::new();
        store.create(row("t-1", "Acme")).await.unwrap();
        store
            .put(TenantRow {
                tenant_id: "t-1".into(),
                name: "Acme Inc".into(),
                locale: "es".into(),
            })
            .await
            .unwrap();
        let r = store.get("t-1").await.unwrap().unwrap();
        assert_eq!(r.name, "Acme Inc");
        assert_eq!(r.locale, "es");
    }
}
