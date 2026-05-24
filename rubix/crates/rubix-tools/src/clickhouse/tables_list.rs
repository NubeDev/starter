//! `rubix.clickhouse.tables.list` — tool dispatch.
//!
//! Read-only: queries `ChWriter::list_tables` and returns the
//! rows sorted by name. The in-memory backing returns the union
//! of marts and TTL-tracked tables with a constant engine name;
//! the CH-backed swap will return real `system.tables` rows.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::tables_list::{
    ClickhouseTableSummary, ClickhouseTablesListRequest, ClickhouseTablesListResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::clickhouse::store::ChWriter;

/// Concrete [`Tool`] for `rubix.clickhouse.tables.list`.
pub struct ClickhouseTablesListTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseTablesListTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseTablesListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.tables.list".to_owned(),
            description: rubix_spi::dto::clickhouse::tables_list::DESCRIPTOR
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
        let _req: ClickhouseTablesListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseTablesListRequest: {e}"),
            })?;

        let rows = self.writer.list_tables().await?;
        let tables: Vec<ClickhouseTableSummary> = rows
            .into_iter()
            .map(|t| ClickhouseTableSummary {
                table_name: t.table_name,
                engine: t.engine,
                retention_days: t.retention_days,
                row_count: t.row_count,
            })
            .collect();
        let count = tables.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.clickhouse.tables.listed")
                .expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = ClickhouseTablesListResponse {
            summary,
            count,
            tables,
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
    async fn empty_writer_lists_zero_tables() {
        let tool = ClickhouseTablesListTool::new(Arc::new(InMemoryChWriter::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseTablesListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.clickhouse.tables.listed",
        );
        assert_eq!(resp.count, 0);
    }

    #[tokio::test]
    async fn union_of_marts_and_ttl_tables_comes_back_sorted() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_mart("z_mart", "CREATE TABLE z_mart () ENGINE = MergeTree");
        writer.seed_retention("a_ttl", 30);
        // Overlap: a mart with TTL appears once with retention.
        writer.seed_mart("m_both", "CREATE TABLE m_both () ENGINE = MergeTree");
        writer.seed_retention("m_both", 90);
        let tool = ClickhouseTablesListTool::new(writer);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseTablesListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 3);
        let names: Vec<&str> = resp.tables.iter().map(|t| t.table_name.as_str()).collect();
        assert_eq!(names, vec!["a_ttl", "m_both", "z_mart"]);
        let m_both = resp.tables.iter().find(|t| t.table_name == "m_both").unwrap();
        assert_eq!(m_both.retention_days, Some(90));
    }
}
