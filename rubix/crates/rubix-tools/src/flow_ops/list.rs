//! `rubix.flow_ops.list` — tool dispatch.
//!
//! Read-only verb: queries the shared [`FlowDefStore`] for every
//! row where `superseded_at IS NULL`, sorts by `flow_id` for stable
//! rendering, and emits a `Diagnostic` keyed `rubix.flow.listed`.
//! No [`ReversibleTool`] impl — the verb makes no state change to
//! record. See [docs/design/flows/](../../../../docs/design/flows/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::flow_ops::list::{FlowListItem, FlowListRequest, FlowListResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::flow_ops::store::FlowDefStore;

/// Concrete [`Tool`] for `rubix.flow_ops.list`.
pub struct FlowListTool {
    store: Arc<dyn FlowDefStore>,
}

impl FlowListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn FlowDefStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for FlowListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.flow_ops.list".to_owned(),
            description: rubix_spi::dto::flow_ops::list::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let _req: FlowListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("FlowListRequest: {e}"),
            })?;

        let mut rows = self.store.list_live().await?;
        rows.sort_by(|a, b| a.flow_id.cmp(&b.flow_id));
        let flows: Vec<FlowListItem> = rows
            .into_iter()
            .map(|r| FlowListItem {
                flow_id: r.flow_id,
                revision_id: r.revision_id,
                body_yaml: r.body_yaml,
            })
            .collect();
        let count = flows.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.flow.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = FlowListResponse {
            summary,
            count,
            flows,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow_ops::store::InMemoryFlowDefStore;

    #[tokio::test]
    async fn empty_store_lists_zero_flows() {
        let tool = FlowListTool::new(Arc::new(InMemoryFlowDefStore::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.flow.listed");
        assert_eq!(resp.count, 0);
    }

    #[tokio::test]
    async fn live_rows_come_back_sorted_by_flow_id() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        store.insert_revision("com.x.zed", "id: com.x.zed", 1).await.unwrap();
        store.insert_revision("com.x.ada", "id: com.x.ada", 2).await.unwrap();
        store.insert_revision("com.x.kay", "id: com.x.kay", 3).await.unwrap();
        // Add a superseded row — it MUST NOT appear in the list.
        store.insert_revision("com.x.ada", "id: com.x.ada", 4).await.unwrap();
        let tool = FlowListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let ids: Vec<&str> = resp.flows.iter().map(|f| f.flow_id.as_str()).collect();
        assert_eq!(ids, vec!["com.x.ada", "com.x.kay", "com.x.zed"]);
    }

    #[tokio::test]
    async fn body_yaml_is_returned_inline_on_every_row() {
        let store = Arc::new(InMemoryFlowDefStore::new());
        store
            .insert_revision("com.x.alpha", "id: com.x.alpha\nbody: yes\n", 10)
            .await
            .unwrap();
        store
            .insert_revision("com.x.beta", "id: com.x.beta\nbody: yes\n", 20)
            .await
            .unwrap();
        let tool = FlowListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: FlowListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.flows.len(), 2);
        for f in &resp.flows {
            assert!(
                f.body_yaml.contains(&format!("id: {}", f.flow_id)),
                "body_yaml must round-trip from the live row (got {:?} for {})",
                f.body_yaml,
                f.flow_id,
            );
        }
    }
}
