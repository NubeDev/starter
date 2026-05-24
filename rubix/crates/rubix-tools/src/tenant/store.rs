//! In-memory backing store for the `tenant.list` verb.
//!
//! `tenant.list` is read-only in this phase (no `Reversible`); the
//! store therefore exposes only `insert` + `list`. The production
//! binary swaps in a PG-backed impl that reads from the tenants
//! dimension table.
//!
//! See [docs/design/user-admin/](../../../../docs/design/user-admin/README.md)
//! §"tenant.list" for the verb contract.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_spi::error::Result;

/// One tenant row as surfaced by `rubix.tenant.list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TenantRow {
    /// Stable id.
    pub tenant_id: String,
    /// Human-facing name.
    pub name: String,
    /// IETF locale tag (e.g. `en`, `es`). Returned so the caller can
    /// localise per-tenant follow-up prompts.
    pub locale: String,
}

/// Persistence surface `tenant.list` targets.
#[async_trait]
pub trait TenantStore: Send + Sync {
    /// List all tenant rows. Order is unspecified — callers sort if
    /// they need stability.
    async fn list(&self) -> Result<Vec<TenantRow>>;
}

/// In-memory [`TenantStore`] for tests and the in-process smoke
/// session.
#[derive(Default, Clone)]
pub struct InMemoryTenantStore {
    rows: Arc<Mutex<Vec<TenantRow>>>,
}

impl InMemoryTenantStore {
    /// New empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed with the supplied rows.
    pub fn seeded(rows: Vec<TenantRow>) -> Self {
        Self {
            rows: Arc::new(Mutex::new(rows)),
        }
    }

    /// Append a row.
    pub fn insert(&self, row: TenantRow) {
        self.rows
            .lock()
            .expect("TenantStore mutex poisoned")
            .push(row);
    }
}

#[async_trait]
impl TenantStore for InMemoryTenantStore {
    async fn list(&self) -> Result<Vec<TenantRow>> {
        Ok(self
            .rows
            .lock()
            .expect("TenantStore mutex poisoned")
            .clone())
    }
}
