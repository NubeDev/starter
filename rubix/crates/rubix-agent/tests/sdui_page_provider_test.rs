//! Sibling integration coverage for `PgPageProvider`. The unit
//! tests in `src/sdui/page_provider.rs` already cover the bundled-
//! tenant lookup against an in-memory `DashboardStore`; this file
//! exercises the explicit-tenant builder so per-tenant scoping (the
//! Phase B.3 plumbing) has a regression seam in place.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rubix_agent::sdui::page_provider::PgPageProvider;
use rubix_spi::dashboard::{
    DashboardRevision, DashboardStore, DashboardStoreError, ListFilter, NewRevision,
};
use starter_sdui_routes::PageProvider;

#[derive(Default)]
struct InMemoryStore {
    rows: Mutex<Vec<DashboardRevision>>,
}

impl InMemoryStore {
    fn seed(&self, page_id: &str, tenant: &str) {
        self.rows.lock().unwrap().push(DashboardRevision {
            page_id: page_id.into(),
            revision_id: "r1".into(),
            tenant_id: tenant.into(),
            owner_principal: "op@example.com".into(),
            title: "t".into(),
            tags: vec![],
            body_json: serde_json::json!({
                "ir_version": starter_ui_ir::IR_VERSION,
                "root": { "type": "page", "id": "p1", "children": [] }
            }),
            created_by: "op@example.com".into(),
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

#[tokio::test]
async fn explicit_tenant_provider_resolves_only_matching_tenant() {
    let store = Arc::new(InMemoryStore::default());
    store.seed("dashboard.energy", "tenant-a");
    store.seed("dashboard.energy", "tenant-b");

    let p_a = PgPageProvider::for_tenant(store.clone(), "tenant-a");
    let p_c = PgPageProvider::for_tenant(store, "tenant-c");

    assert!(p_a.lookup_page("dashboard.energy").await.is_some());
    assert!(p_c.lookup_page("dashboard.energy").await.is_none());
}
