//! `rubix.audit.policy.list` — tool dispatch.
//!
//! Read-only: walks the [`AuditPolicyStore`] and surfaces every
//! row. Kinds absent from the store are implicitly unbounded and
//! are not represented here. No [`ReversibleTool`] impl \u{2014}
//! list verbs make no state change.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::audit::policy_list::{
    AuditPolicyEntry, AuditPolicyListRequest, AuditPolicyListResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::audit::store::AuditPolicyStore;

/// Concrete [`Tool`] for `rubix.audit.policy.list`.
pub struct AuditPolicyListTool {
    store: Arc<dyn AuditPolicyStore>,
}

impl AuditPolicyListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn AuditPolicyStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for AuditPolicyListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.audit.policy.list".to_owned(),
            description: rubix_spi::dto::audit::policy_list::DESCRIPTOR
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
        let _req: AuditPolicyListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("AuditPolicyListRequest: {e}"),
            })?;

        // Store already returns rows sorted by kind \u{2014} trust
        // the contract rather than re-sorting on the verb side.
        let rows = self.store.list().await?;
        let entries: Vec<AuditPolicyEntry> = rows
            .into_iter()
            .map(|r| AuditPolicyEntry {
                resource_kind: r.resource_kind,
                max_age_days: r.max_age_days,
                updated_at_ms: r.updated_at_ms,
            })
            .collect();
        let count = entries.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.audit.policy.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = AuditPolicyListResponse { summary, entries };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::store::{AuditPolicyRow, InMemoryAuditPolicyStore};

    #[tokio::test]
    async fn list_empty_store_returns_zero_entries() {
        let store = Arc::new(InMemoryAuditPolicyStore::new());
        let tool = AuditPolicyListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: AuditPolicyListResponse = serde_json::from_value(out).unwrap();
        assert!(resp.entries.is_empty());
        assert_eq!(resp.summary.code.as_str(), "rubix.audit.policy.listed");
    }

    #[tokio::test]
    async fn list_returns_sorted_entries() {
        let store = Arc::new(InMemoryAuditPolicyStore::new());
        store
            .put(AuditPolicyRow {
                resource_kind: "user".into(),
                max_age_days: None,
                updated_at_ms: 100,
            })
            .await
            .unwrap();
        store
            .put(AuditPolicyRow {
                resource_kind: "flow_def".into(),
                max_age_days: Some(30),
                updated_at_ms: 200,
            })
            .await
            .unwrap();
        let tool = AuditPolicyListTool::new(store);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: AuditPolicyListResponse = serde_json::from_value(out).unwrap();
        let kinds: Vec<_> = resp.entries.iter().map(|e| e.resource_kind.as_str()).collect();
        assert_eq!(kinds, ["flow_def", "user"]);
        assert_eq!(resp.entries[0].max_age_days, Some(30));
        assert_eq!(resp.entries[1].max_age_days, None);
    }
}
