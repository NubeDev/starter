//! `rubix.dashboard.list` — tool dispatch.
//!
//! Read-only verb: queries [`DashboardStore::list_active`] for the
//! caller's tenant filtered by optional tag-overlap and/or owner,
//! sorts rows by `page_id` for stable rendering, and emits a
//! `Diagnostic` keyed `rubix.dashboard.listed`. No
//! [`Reversible`](starter_spi::changelog::Reversible) impl — the
//! verb makes no state change to record.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dashboard::{DashboardStore, ListFilter};
use rubix_spi::dto::dashboard::list::{
    DashboardSummary, ListDashboardsRequest, ListDashboardsResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

/// Concrete [`Tool`] for `rubix.dashboard.list`.
pub struct DashboardListTool {
    store: Arc<dyn DashboardStore>,
}

impl DashboardListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn DashboardStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DashboardListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.dashboard.list".to_owned(),
            description: rubix_spi::dto::dashboard::list::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string" },
                    "tags_any":  { "type": "array", "items": { "type": "string" } },
                    "owner":     { "type": ["string", "null"] }
                },
                "required": ["tenant_id"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: ListDashboardsRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ListDashboardsRequest: {e}"),
            })?;

        let filter = ListFilter {
            tags_any: req.tags_any,
            owner: req.owner,
        };

        let mut rows = self
            .store
            .list_active(&req.tenant_id, &filter)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;
        rows.sort_by(|a, b| a.page_id.cmp(&b.page_id));

        let items: Vec<DashboardSummary> = rows
            .into_iter()
            .map(|r| DashboardSummary {
                page_id: r.page_id,
                revision_id: r.revision_id,
                title: r.title,
                tags: r.tags,
                owner_principal: r.owner_principal,
                updated_at: r.created_at,
            })
            .collect();
        let count = items.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.dashboard.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = ListDashboardsResponse {
            summary,
            count,
            items,
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
    use rubix_spi::dashboard::{DashboardRevision, DashboardStoreError, NewRevision};
    use std::sync::Mutex;

    #[derive(Default)]
    struct InMemoryStore {
        rows: Mutex<Vec<DashboardRevision>>,
    }

    impl InMemoryStore {
        fn seed(&self, page_id: &str, tenant: &str, owner: &str, tags: Vec<String>) {
            self.rows.lock().unwrap().push(DashboardRevision {
                page_id: page_id.into(),
                revision_id: format!("rev-{page_id}"),
                tenant_id: tenant.into(),
                owner_principal: owner.into(),
                title: format!("Title for {page_id}"),
                tags,
                body_json: serde_json::json!({}),
                created_by: owner.into(),
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
            _: &str,
            _: &str,
        ) -> std::result::Result<Option<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
        async fn list_active(
            &self,
            tenant_id: &str,
            filter: &ListFilter,
        ) -> std::result::Result<Vec<DashboardRevision>, DashboardStoreError> {
            let rows = self.rows.lock().unwrap();
            let out: Vec<DashboardRevision> = rows
                .iter()
                .filter(|r| r.tenant_id == tenant_id)
                .filter(|r| {
                    filter.tags_any.is_empty() || r.tags.iter().any(|t| filter.tags_any.contains(t))
                })
                .filter(|r| match &filter.owner {
                    None => true,
                    Some(o) => &r.owner_principal == o,
                })
                .cloned()
                .collect();
            Ok(out)
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
    async fn empty_store_lists_zero_pages() {
        let tool = DashboardListTool::new(Arc::new(InMemoryStore::default()));
        let out = tool
            .invoke(serde_json::json!({ "tenant_id": "system" }))
            .await
            .unwrap();
        let resp: ListDashboardsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.dashboard.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.items.is_empty());
    }

    #[tokio::test]
    async fn live_rows_come_back_sorted_by_page_id_and_filtered_by_tenant() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.zed", "system", "system", vec![]);
        store.seed("dashboard.ada", "system", "system", vec![]);
        store.seed("dashboard.kay", "system", "system", vec![]);
        // Different tenant — must not surface.
        store.seed("dashboard.other", "tenant-x", "alice", vec![]);
        let tool = DashboardListTool::new(store);
        let out = tool
            .invoke(serde_json::json!({ "tenant_id": "system" }))
            .await
            .unwrap();
        let resp: ListDashboardsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let ids: Vec<&str> = resp.items.iter().map(|s| s.page_id.as_str()).collect();
        assert_eq!(ids, vec!["dashboard.ada", "dashboard.kay", "dashboard.zed"]);
    }

    #[tokio::test]
    async fn tags_any_filter_narrows_results() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.a", "system", "system", vec!["bundled".into()]);
        store.seed("dashboard.b", "system", "system", vec!["custom".into()]);
        let tool = DashboardListTool::new(store);
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id": "system",
                "tags_any": ["bundled"]
            }))
            .await
            .unwrap();
        let resp: ListDashboardsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 1);
        assert_eq!(resp.items[0].page_id, "dashboard.a");
    }

    #[tokio::test]
    async fn owner_filter_narrows_results() {
        let store = Arc::new(InMemoryStore::default());
        store.seed("dashboard.a", "tenant-1", "alice", vec![]);
        store.seed("dashboard.b", "tenant-1", "bob", vec![]);
        let tool = DashboardListTool::new(store);
        let out = tool
            .invoke(serde_json::json!({
                "tenant_id": "tenant-1",
                "owner":     "bob"
            }))
            .await
            .unwrap();
        let resp: ListDashboardsResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 1);
        assert_eq!(resp.items[0].owner_principal, "bob");
    }
}
