//! `rubix.warehouse.rule.list` — enumerate continuous aggregates
//! tagged as "rules" (derived-state views).
//!
//! Per the warehouse-engine-swap proposal §"The mart / continuous
//! aggregate translation", rules and marts are both continuous
//! aggregates on TimescaleDB. The distinction in this build is
//! naming convention: rules are caggs whose `view_name` ends in
//! `_rule` or starts with `rule_`; everything else surfaces via
//! `mart.list`. When the agent gains a registration table this
//! probe is replaced; until then naming is the explicit source of
//! truth.
//!
//! DDL is read via `pg_get_viewdef` joined with
//! `timescaledb_information.continuous_aggregates`.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::rule_list::{
    ClickhouseRuleSummary, WarehouseRuleListRequest, WarehouseRuleListResponse,
};
use serde_json::Value;
use sqlx::Row;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::WarehouseClient;

pub struct WarehouseRuleListTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseRuleListTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseRuleListTool").finish()
    }
}

impl WarehouseRuleListTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

/// True when the cagg name follows the rule convention.
pub(crate) fn is_rule_name(view_name: &str) -> bool {
    view_name.ends_with("_rule") || view_name.starts_with("rule_")
}

#[async_trait]
impl Tool for WarehouseRuleListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.rule.list".to_owned(),
            description: rubix_spi::dto::warehouse::rule_list::DESCRIPTOR
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
        let _req: WarehouseRuleListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseRuleListRequest: {e}"),
            })?;

        let rows = sqlx::query(
            "SELECT \
               ca.view_name, \
               pg_get_viewdef(format('%I.%I', ca.view_schema, ca.view_name)::regclass) \
                 AS view_definition \
             FROM timescaledb_information.continuous_aggregates AS ca \
             ORDER BY ca.view_name ASC",
        )
        .fetch_all(self.client.pool())
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;

        let mut rules: Vec<ClickhouseRuleSummary> = rows
            .into_iter()
            .filter_map(|r| {
                let view_name: String = r.try_get("view_name").ok()?;
                if !is_rule_name(&view_name) {
                    return None;
                }
                let ddl: Option<String> = r.try_get("view_definition").ok();
                Some(ClickhouseRuleSummary {
                    rule_name: view_name,
                    ddl,
                })
            })
            .collect();
        rules.sort_by(|a, b| a.rule_name.cmp(&b.rule_name));
        let count = rules.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.warehouse.rule.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = WarehouseRuleListResponse {
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

    #[test]
    fn rule_naming_convention() {
        assert!(is_rule_name("samples_1h_rule"));
        assert!(is_rule_name("rule_disk_high"));
        assert!(!is_rule_name("samples_1h_mart"));
        assert!(!is_rule_name("samples_1h"));
    }
}
