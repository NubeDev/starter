//! `rubix.clickhouse.mart.list` — tool dispatch.
//!
//! Read-only: queries `ChWriter::list_marts` and returns the rows
//! sorted by name. No state change, no `ReversibleTool` impl.

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::mart_list::{
    ClickhouseMartListRequest, ClickhouseMartListResponse, ClickhouseMartSummary,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::clickhouse::store::ChWriter;

/// Concrete [`Tool`] for `rubix.clickhouse.mart.list`.
pub struct ClickhouseMartListTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseMartListTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseMartListTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.mart.list".to_owned(),
            description: rubix_spi::dto::clickhouse::mart_list::DESCRIPTOR
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
        let _req: ClickhouseMartListRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseMartListRequest: {e}"),
            })?;

        let snapshots = self.writer.list_marts().await?;
        let marts: Vec<ClickhouseMartSummary> = snapshots
            .into_iter()
            .map(|s| ClickhouseMartSummary {
                mart_name: s.mart_name,
                ddl: s.ddl,
            })
            .collect();
        let count = marts.len();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.clickhouse.mart.listed")
                .expect("hard-coded key parses"),
        )
        .with_param("count", DiagnosticParam::I64(count as i64));

        let response = ClickhouseMartListResponse {
            summary,
            count,
            marts,
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
    async fn empty_writer_lists_zero_marts() {
        let tool = ClickhouseMartListTool::new(Arc::new(InMemoryChWriter::new()));
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseMartListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.clickhouse.mart.listed");
        assert_eq!(resp.count, 0);
    }

    #[tokio::test]
    async fn marts_come_back_sorted() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_mart("z_mart", "CREATE TABLE z_mart () ENGINE = MergeTree");
        writer.seed_mart("a_mart", "CREATE TABLE a_mart () ENGINE = MergeTree");
        let tool = ClickhouseMartListTool::new(writer);
        let out = tool.invoke(serde_json::json!({})).await.unwrap();
        let resp: ClickhouseMartListResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.count, 2);
        let names: Vec<&str> = resp.marts.iter().map(|r| r.mart_name.as_str()).collect();
        assert_eq!(names, vec!["a_mart", "z_mart"]);
    }
}
