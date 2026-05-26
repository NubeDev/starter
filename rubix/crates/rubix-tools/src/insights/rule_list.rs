//! `rubix.insights.rule.list` — tool dispatch.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::insights::rule_list::{
    InsightsRuleListRequest, InsightsRuleListResponse, InsightsRuleSummary,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::insights::store::InsightsRuleStore;

/// Concrete [`Tool`] for `rubix.insights.rule.list`.
pub struct InsightsRuleListTool {
    store: Arc<dyn InsightsRuleStore>,
}

impl InsightsRuleListTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn InsightsRuleStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for InsightsRuleListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.insights.rule.list".to_owned(),
            description: rubix_spi::dto::insights::rule_list::DESCRIPTOR
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
        let _req: InsightsRuleListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("InsightsRuleListRequest: {e}"),
            })?;

        let rows = self.store.list().await?;
        let rules: Vec<InsightsRuleSummary> = rows
            .into_iter()
            .map(|r| InsightsRuleSummary {
                rule_id: r.rule_id,
                name: r.name,
                enabled: r.enabled,
                body_yaml: Some(r.body_yaml),
                updated_at_ms: r.updated_at_ms,
            })
            .collect();
        let count = rules.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.insights.rule.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = InsightsRuleListResponse {
            summary,
            count,
            rules,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insights::store::InMemoryInsightsStore;

    #[tokio::test]
    async fn empty_store_lists_zero_rules() {
        let tool = InsightsRuleListTool::new(Arc::new(InMemoryInsightsStore::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: InsightsRuleListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.rules.is_empty());
    }
}
