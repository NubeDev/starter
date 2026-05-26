//! `rubix.insights.rule.{enable, disable}` — two tools sharing the
//! same request/response shape and store path. The diagnostic
//! `code` differs so the UI can render the right toast.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::insights::rule_toggle::{
    InsightsRuleToggleRequest, InsightsRuleToggleResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::insights::store::{now_epoch_ms, InsightsRuleStore, ToggleOutcome};

/// Concrete [`Tool`] for `rubix.insights.rule.enable`.
pub struct InsightsRuleEnableTool {
    store: Arc<dyn InsightsRuleStore>,
}

impl InsightsRuleEnableTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn InsightsRuleStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for InsightsRuleEnableTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.insights.rule.enable".to_owned(),
            description: rubix_spi::dto::insights::rule_toggle::DESCRIPTOR_ENABLE
                .purpose
                .to_owned(),
            input_schema: toggle_schema(),
        }
    }
    async fn invoke(&self, input: Value) -> Result<Value> {
        run_toggle(self.store.as_ref(), input, true).await
    }
}

/// Concrete [`Tool`] for `rubix.insights.rule.disable`.
pub struct InsightsRuleDisableTool {
    store: Arc<dyn InsightsRuleStore>,
}

impl InsightsRuleDisableTool {
    /// Wrap the shared store.
    pub fn new(store: Arc<dyn InsightsRuleStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for InsightsRuleDisableTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.insights.rule.disable".to_owned(),
            description: rubix_spi::dto::insights::rule_toggle::DESCRIPTOR_DISABLE
                .purpose
                .to_owned(),
            input_schema: toggle_schema(),
        }
    }
    async fn invoke(&self, input: Value) -> Result<Value> {
        run_toggle(self.store.as_ref(), input, false).await
    }
}

fn toggle_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "rule_id": { "type": "string", "minLength": 1 }
        },
        "required": ["rule_id"],
        "additionalProperties": false
    })
}

async fn run_toggle(store: &dyn InsightsRuleStore, input: Value, target: bool) -> Result<Value> {
    let req: InsightsRuleToggleRequest =
        serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("InsightsRuleToggleRequest: {e}"),
        })?;
    let toggled_at_ms = now_epoch_ms();
    let outcome = store
        .set_enabled(&req.rule_id, target, toggled_at_ms)
        .await?;
    let (code, enabled) = match outcome {
        ToggleOutcome::Applied if target => ("rubix.insights.rule.enabled", true),
        ToggleOutcome::Applied => ("rubix.insights.rule.disabled", false),
        ToggleOutcome::NotFound => ("rubix.insights.rule.not_found", target),
    };
    let summary = Diagnostic::new(MessageKey::parse(code).expect("hard-coded key parses"))
        .with_param("rule", DiagnosticParam::String(req.rule_id.clone()));
    let response = InsightsRuleToggleResponse {
        summary,
        rule_id: req.rule_id,
        enabled,
        toggled_at_ms,
    };
    serde_json::to_value(response).map_err(|e| Error::Internal {
        source: Box::new(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::insights::store::InMemoryInsightsStore;

    #[tokio::test]
    async fn enable_then_disable_round_trip() {
        let store = Arc::new(InMemoryInsightsStore::new());
        // seed via the upsert path
        store.upsert("r1", "body", 1).await.unwrap();
        store.set_enabled("r1", false, 2).await.unwrap();

        let enable = InsightsRuleEnableTool::new(store.clone());
        let out = enable
            .invoke(serde_json::json!({"rule_id": "r1"}))
            .await
            .unwrap();
        let resp: InsightsRuleToggleResponse = serde_json::from_value(out).unwrap();
        assert!(resp.enabled);
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.enabled");

        let disable = InsightsRuleDisableTool::new(store.clone());
        let out = disable
            .invoke(serde_json::json!({"rule_id": "r1"}))
            .await
            .unwrap();
        let resp: InsightsRuleToggleResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.enabled);
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.disabled");
    }

    #[tokio::test]
    async fn unknown_rule_id_surfaces_not_found_diagnostic() {
        let store = Arc::new(InMemoryInsightsStore::new());
        let enable = InsightsRuleEnableTool::new(store);
        let out = enable
            .invoke(serde_json::json!({"rule_id": "ghost"}))
            .await
            .unwrap();
        let resp: InsightsRuleToggleResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.insights.rule.not_found");
    }
}
