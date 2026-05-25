//! `rubix.insights.rule.create` — tool dispatch.
//!
//! Idempotent upsert: existing ids have their body replaced and
//! surface a `rubix.insights.rule.replaced` diagnostic. Fresh ids
//! surface `rubix.insights.rule.created` and start `enabled = true`.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::insights::rule_create::{
    InsightsRuleCreateRequest, InsightsRuleCreateResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::insights::store::{now_epoch_ms, InsightsRuleStore, UpsertOutcome};

/// Concrete [`Tool`] for `rubix.insights.rule.create`.
pub struct InsightsRuleCreateTool {
    store: Arc<dyn InsightsRuleStore>,
}

impl InsightsRuleCreateTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn InsightsRuleStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for InsightsRuleCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.insights.rule.create".to_owned(),
            description: rubix_spi::dto::insights::rule_create::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "rule_id":   { "type": "string", "minLength": 1 },
                    "body_yaml": { "type": "string", "minLength": 1 }
                },
                "required": ["rule_id", "body_yaml"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: InsightsRuleCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("InsightsRuleCreateRequest: {e}"),
            })?;
        if req.rule_id.trim().is_empty() {
            return Err(Error::Invalid {
                message: "InsightsRuleCreateRequest: rule_id must not be blank".to_owned(),
            });
        }
        let created_at_ms = now_epoch_ms();
        let outcome = self
            .store
            .upsert(&req.rule_id, &req.body_yaml, created_at_ms)
            .await?;
        let code = match outcome {
            UpsertOutcome::Created => "rubix.insights.rule.created",
            UpsertOutcome::Replaced => "rubix.insights.rule.replaced",
        };
        let summary = Diagnostic::new(
            MessageKey::parse(code).expect("hard-coded key parses"),
        )
        .with_param("rule", DiagnosticParam::String(req.rule_id.clone()));

        let response = InsightsRuleCreateResponse {
            summary,
            rule_id: req.rule_id,
            created_at_ms,
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
    async fn fresh_id_emits_created_diagnostic() {
        let store = Arc::new(InMemoryInsightsStore::new());
        let tool = InsightsRuleCreateTool::new(store.clone());
        let out = tool
            .invoke(serde_json::json!({
                "rule_id": "disk-high",
                "body_yaml": "when: disk.used_pct > 90\n",
            }))
            .await
            .unwrap();
        let resp: InsightsRuleCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.created");
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn second_write_emits_replaced_diagnostic() {
        let store = Arc::new(InMemoryInsightsStore::new());
        let tool = InsightsRuleCreateTool::new(store.clone());
        let input = serde_json::json!({"rule_id": "x", "body_yaml": "a"});
        tool.invoke(input.clone()).await.unwrap();
        let out = tool.invoke(input).await.unwrap();
        let resp: InsightsRuleCreateResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.replaced");
    }

    #[tokio::test]
    async fn blank_rule_id_is_rejected() {
        let tool = InsightsRuleCreateTool::new(Arc::new(InMemoryInsightsStore::new()));
        let err = tool
            .invoke(serde_json::json!({"rule_id": "  ", "body_yaml": "x"}))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }
}
