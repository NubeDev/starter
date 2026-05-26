//! `PgPageProvider` — rubix's impl of
//! [`starter_sdui_routes::PageProvider`].
//!
//! Looks the `page_ref` up via [`PgDashboardStore::get_active`] and
//! deserialises the stored `body_json` to a
//! [`starter_ui_ir::ComponentTree`]. The Phase B.2 cache + NOTIFY
//! listener will wrap this provider; Phase B.1 keeps the trait impl
//! cache-less so the cache layer's tests can assert it actually
//! caches.
//!
//! ## Tenant scoping
//!
//! [`PageProvider::lookup_page`] is single-argument by design — the
//! upstream router does not know rubix's tenancy model. Phase B.1
//! resolves bundled (`tenant_id = BUNDLED_TENANT`) pages only. Per-
//! tenant scoping lands with the resolver's principal-aware
//! middleware in stage B.3; until then this provider returns the
//! `system` row for `page_ref`. That is exactly the behaviour the
//! seeded `disk-overview` page (Phase A.5) needs at boot.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, BUNDLED_TENANT};
use starter_sdui_routes::PageProvider;
use starter_ui_ir::ComponentTree;
use tracing::warn;

/// Page provider backed by [`DashboardStore`].
#[derive(Clone)]
pub struct PgPageProvider {
    store: Arc<dyn DashboardStore>,
    tenant: String,
}

impl std::fmt::Debug for PgPageProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgPageProvider")
            .field("tenant", &self.tenant)
            .finish()
    }
}

impl PgPageProvider {
    /// Build a provider scoped to the bundled tenant. Production
    /// callers should use this; per-tenant providers land in B.3.
    pub fn bundled(store: Arc<dyn DashboardStore>) -> Self {
        Self {
            store,
            tenant: BUNDLED_TENANT.to_string(),
        }
    }

    /// Build a provider scoped to an explicit tenant id. Tests use
    /// this to exercise multi-tenant lookups.
    pub fn for_tenant(store: Arc<dyn DashboardStore>, tenant: impl Into<String>) -> Self {
        Self {
            store,
            tenant: tenant.into(),
        }
    }
}

#[async_trait]
impl PageProvider for PgPageProvider {
    async fn lookup_page(&self, page_ref: &str) -> Option<ComponentTree> {
        let row = match self.store.get_active(&self.tenant, page_ref).await {
            Ok(Some(row)) => row,
            Ok(None) => return None,
            Err(err) => {
                warn!(
                    target: "rubix.sdui",
                    tenant = %self.tenant,
                    page_ref,
                    error = %err,
                    "page provider: store lookup failed",
                );
                return None;
            }
        };
        match serde_json::from_value::<ComponentTree>(row.body_json) {
            Ok(tree) => Some(tree),
            Err(err) => {
                warn!(
                    target: "rubix.sdui",
                    tenant = %self.tenant,
                    page_ref,
                    error = %err,
                    "page provider: stored body_json does not deserialise to ComponentTree",
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rubix_spi::dashboard::{DashboardRevision, DashboardStoreError, ListFilter, NewRevision};
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryStore {
        rows: Mutex<Vec<DashboardRevision>>,
    }

    impl InMemoryStore {
        fn seed(&self, page_id: &str, tenant: &str, body: serde_json::Value) {
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: "r1".into(),
                tenant_id: tenant.into(),
                owner_principal: "system".into(),
                title: "t".into(),
                tags: vec![],
                body_json: body,
                created_by: "system".into(),
                created_at: "2026-05-25T00:00:00Z".into(),
                superseded_at: None,
            });
        }
    }

    #[async_trait]
    impl DashboardStore for InMemoryStore {
        async fn insert_revision(
            &self,
            _: NewRevision,
        ) -> Result<DashboardRevision, DashboardStoreError> {
            unimplemented!()
        }
        async fn get_active(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> Result<Option<DashboardRevision>, DashboardStoreError> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .find(|r| r.tenant_id == tenant_id && r.page_id == page_id)
                .cloned())
        }
        async fn list_active(
            &self,
            _: &str,
            _: &ListFilter,
        ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
        async fn mark_superseded(&self, _: &str, _: &str) -> Result<u64, DashboardStoreError> {
            unimplemented!()
        }
        async fn history(&self, _: &str) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
    }

    fn minimal_page_body() -> serde_json::Value {
        // Minimal valid `ComponentTree` — a bare `page` root. The
        // assertion exercises the real `serde_json::from_value`
        // path so any IR-shape drift trips the test.
        serde_json::json!({
            "ir_version": starter_ui_ir::IR_VERSION,
            "root": { "type": "page", "id": "p1", "children": [] }
        })
    }

    #[tokio::test]
    async fn returns_none_for_missing_page() {
        let store = Arc::new(InMemoryStore::default());
        let p = PgPageProvider::bundled(store);
        assert!(p.lookup_page("dashboard.missing").await.is_none());
    }

    #[tokio::test]
    async fn returns_some_for_seeded_bundled_page() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.test", BUNDLED_TENANT, minimal_page_body());
        let p = PgPageProvider::bundled(store);
        let tree = p.lookup_page("dashboard.test").await;
        assert!(tree.is_some(), "seeded page should deserialise");
    }

    #[tokio::test]
    async fn skips_other_tenants() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.test", "tenant-x", minimal_page_body());
        let p = PgPageProvider::bundled(store);
        assert!(p.lookup_page("dashboard.test").await.is_none());
    }
}
