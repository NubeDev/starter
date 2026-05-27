//! `rubix.warehouse.mart.list` — enumerate continuous aggregates
//! that are marts (history/aggregate tables, not derived-state
//! rules).
//!
//! Sibling of `rule.list`: same data source
//! (`timescaledb_information.continuous_aggregates`), opposite
//! naming filter — anything that is NOT a rule.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::mart_list::{
    ClickhouseMartSummary, WarehouseMartListRequest, WarehouseMartListResponse,
};
use serde_json::Value;
use sqlx::Row;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::WarehouseClient;

use crate::warehouse::rule_list::is_rule_name;

pub struct WarehouseMartListTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseMartListTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseMartListTool").finish()
    }
}

impl WarehouseMartListTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Tool for WarehouseMartListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.mart.list".to_owned(),
            description: rubix_spi::dto::warehouse::mart_list::DESCRIPTOR
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
        let _req: WarehouseMartListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseMartListRequest: {e}"),
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

        let mut marts: Vec<ClickhouseMartSummary> = rows
            .into_iter()
            .filter_map(|r| {
                let view_name: String = r.try_get("view_name").ok()?;
                if is_rule_name(&view_name) {
                    return None;
                }
                let ddl: Option<String> = r.try_get("view_definition").ok();
                Some(ClickhouseMartSummary {
                    mart_name: view_name,
                    ddl,
                })
            })
            .collect();
        marts.sort_by(|a, b| a.mart_name.cmp(&b.mart_name));
        let count = marts.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.warehouse.mart.listed").expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = WarehouseMartListResponse {
            summary,
            count,
            marts,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}
