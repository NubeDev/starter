//! `rubix.dashboard.get` — tool dispatch.
//!
//! Read-only verb: looks the row up via
//! [`DashboardStore::get_active`] and emits one of two diagnostics:
//!
//! - `rubix.dashboard.fetched` on hit (the response carries the
//!   body),
//! - `rubix.dashboard.get.not_found` on miss (every body field is
//!   `None`).
//!
//! No [`Reversible`](starter_spi::changelog::Reversible) impl —
//! the verb makes no state change to record. See
//! `rubix/docs/scope/dashboards/04-tools.md`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::DashboardStore;
use rubix_spi::dto::dashboard::get::{GetDashboardRequest, GetDashboardResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete [`Tool`] for `rubix.dashboard.get`.
pub struct DashboardGetTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardGetTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DashboardGetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.get".to_owned(),
            description: rubix_spi::dto::dashboard::get::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string" },
                    "page_id":   { "type": "string" }
                },
                "required": ["tenant_id", "page_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: GetDashboardRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("GetDashboardRequest: {e}"),
            })?;

        let row = self
            .store
            .get_active(&req.tenant_id, &req.page_id)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let response = match row {
            Some(r) => GetDashboardResponse {
                summary: Diagnostic::new(
                    MessageKey::parse("rubix.dashboard.fetched")
                        .expect("hard-coded key parses"),
                )
                .with_param("page_id", DiagnosticParam::String(r.page_id.clone())),
                page_id: r.page_id,
                revision_id: Some(r.revision_id),
                tenant_id: Some(r.tenant_id),
                owner_principal: Some(r.owner_principal),
                title: Some(r.title),
                tags: r.tags,
                body_json: Some(r.body_json),
                created_by: Some(r.created_by),
                created_at: Some(r.created_at),
            },
            None => GetDashboardResponse {
                summary: Diagnostic::new(
                    MessageKey::parse("rubix.dashboard.get.not_found")
                        .expect("hard-coded key parses"),
                )
                .with_param("page_id", DiagnosticParam::String(req.page_id.clone())),
                page_id: req.page_id,
                revision_id: None,
                tenant_id: None,
                owner_principal: None,
                title: None,
                tags: vec![],
                body_json: None,
                created_by: None,
                created_at: None,
            },
        };

        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rubix_spi::dashboard::{
        DashboardRevision, DashboardStoreError, ListFilter, NewRevision,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryStore {
        rows: Mutex<Vec<DashboardRevision>>,
    }

    impl InMemoryStore {
        fn seed(&self, page_id: &str, tenant: &str) {
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: "rev-1".into(),
                tenant_id: tenant.into(),
                owner_principal: "system".into(),
                title: format!("Title for {page_id}"),
                tags: vec!["bundled".into()],
                body_json: serde_json::json!({ "ir_version": 1, "root": {} }),
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
        ) -> std::result::Result<DashboardRevision, DashboardStoreError> {
            unimplemented!()
        }
        async fn get_active(
            &self,
            tenant_id: &str,
            page_id: &str,
        ) -> std::result::Result<Option<DashboardRevision>, DashboardStoreError> {
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
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
        async fn mark_superseded(
            &self,
            _: &str,
            _: &str,
        ) -> std::result::Result<u64, DashboardStoreError> {
            unimplemented!()
        }
        async fn history(
            &self,
            _: &str,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn returns_fetched_diagnostic_on_hit() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.disk-overview", "system");
        let tool = DashboardGetTool::new(store);
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id": "system",
                "page_id":   "dashboard.disk-overview"
            }))
            .await
            .unwrap();
        let resp: GetDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.fetched");
        assert_eq!(resp.page_id, "dashboard.disk-overview");
        assert_eq!(resp.revision_id.as_deref(), Some("rev-1"));
        assert!(resp.body_json.is_some());
        assert_eq!(resp.tags, vec!["bundled".to_string()]);
    }

    #[tokio::test]
    async fn returns_not_found_diagnostic_on_miss() {
        let tool = DashboardGetTool::new(Arc::new(InMemoryStore::default()));
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id": "system",
                "page_id":   "dashboard.missing"
            }))
            .await
            .unwrap();
        let resp: GetDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.get.not_found");
        assert_eq!(resp.page_id, "dashboard.missing");
        assert!(resp.revision_id.is_none());
        assert!(resp.body_json.is_none());
    }

    #[tokio::test]
    async fn miss_when_tenant_does_not_match() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.disk-overview", "tenant-a");
        let tool = DashboardGetTool::new(store);
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id": "tenant-b",
                "page_id":   "dashboard.disk-overview"
            }))
            .await
            .unwrap();
        let resp: GetDashboardResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.get.not_found");
    }
}
