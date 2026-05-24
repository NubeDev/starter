//! `rubix.tenant.list` — tool dispatch.
//!
//! Read-only verb: queries the shared [`TenantStore`], sorts the rows
//! by name for stable rendering, and emits a `Diagnostic` keyed
//! `rubix.tenant.listed`. No [`ReversibleTool`] impl — the verb makes
//! no state change to record. See
//! [docs/design/user-admin/](../../../../docs/design/user-admin/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::tenant::list::{TenantListItem, TenantListRequest, TenantListResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::tenant::store::TenantStore;

/// Concrete [`Tool`] for `rubix.tenant.list`.
pub struct TenantListTool {
    store: Arc<dyn TenantStore>,
}

impl TenantListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn TenantStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for TenantListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.tenant.list".to_owned(),
            description: rubix_spi::dto::tenant::list::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _req: TenantListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("TenantListRequest: {e}"),
            })?;

        let mut rows = self.store.list().await?;
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        let tenants: Vec<TenantListItem> = rows
            .into_iter()
            .map(|r| TenantListItem {
                tenant_id: r.tenant_id,
                name: r.name,
                locale: r.locale,
            })
            .collect();
        let count = tenants.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.tenant.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = TenantListResponse {
            summary,
            count,
            tenants,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant::store::{InMemoryTenantStore, TenantRow};

    fn row(id: &str, name: &str) -> TenantRow {
        TenantRow {
            tenant_id: id.into(),
            name: name.into(),
            locale: "en".into(),
        }
    }

    #[tokio::test]
    async fn empty_store_lists_zero_tenants() {
        let tool = TenantListTool::new(Arc::new(InMemoryTenantStore::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TenantListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.tenant.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.tenants.is_empty());
    }

    #[tokio::test]
    async fn rows_come_back_sorted_by_name() {
        let store = Arc::new(InMemoryTenantStore::seeded(vec![
            row("t-2", "Zenith"),
            row("t-1", "Acme"),
            row("t-3", "Kepler"),
        ]));
        let tool = TenantListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: TenantListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let names: Vec<&str> = resp.tenants.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["Acme", "Kepler", "Zenith"]);
    }
}
