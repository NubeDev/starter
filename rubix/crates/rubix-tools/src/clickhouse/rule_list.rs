//! `rubix.clickhouse.rule.list` — tool dispatch.
//!
//! Read-only: queries `ChWriter::list_rules` and returns the rows
//! sorted by name. No state change, no `ReversibleTool` impl.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::rule_list::{
    ClickhouseRuleListRequest, ClickhouseRuleListResponse, ClickhouseRuleSummary,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::clickhouse::store::ChWriter;

/// Concrete [`Tool`] for `rubix.clickhouse.rule.list`.
pub struct ClickhouseRuleListTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseRuleListTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseRuleListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.rule.list".to_owned(),
            description: rubix_spi::dto::clickhouse::rule_list::DESCRIPTOR
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
        let _req: ClickhouseRuleListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseRuleListRequest: {e}"),
            })?;

        let snapshots = self.writer.list_rules().await?;
        let rules: Vec<ClickhouseRuleSummary> = snapshots
            .into_iter()
            .map(|s| ClickhouseRuleSummary {
                rule_name: s.rule_name,
                ddl: s.ddl,
            })
            .collect();
        let count = rules.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.clickhouse.rule.listed")
                .expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = ClickhouseRuleListResponse {
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
    use crate::clickhouse::store::InMemoryChWriter;

    #[tokio::test]
    async fn empty_writer_lists_zero_rules() {
        let tool = ClickhouseRuleListTool::new(Arc::new(InMemoryChWriter::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseRuleListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.clickhouse.rule.listed");
        assert_eq!(resp.count, 0);
        assert!(resp.rules.is_empty());
    }

    #[tokio::test]
    async fn rules_come_back_sorted() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer
            .apply_rule_ddl("z_view", "CREATE MATERIALIZED VIEW z_view AS SELECT 1")
            .await
            .unwrap();
        writer
            .apply_rule_ddl("a_view", "CREATE MATERIALIZED VIEW a_view AS SELECT 1")
            .await
            .unwrap();
        let tool = ClickhouseRuleListTool::new(writer);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseRuleListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 2);
        let names: Vec<&str> = resp.rules.iter().map(|r| r.rule_name.as_str()).collect();
        assert_eq!(names, vec!["a_view", "z_view"]);
    }
}
