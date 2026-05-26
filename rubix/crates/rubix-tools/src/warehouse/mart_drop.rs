//! `rubix.warehouse.mart.drop` — tool dispatch.
//!
//! Idempotent destructive verb: probes the mart via
//! `WarehouseWriter::show_create_mart`, then walks the
//! "restore-to-absent" path of `restore_mart`. Returns
//! `was_present = false` and a `rubix.warehouse.mart.absent`
//! diagnostic when the mart did not exist.
//!
//! This verb is **not** routed through `ReversibleTool` today —
//! the rubix-agent UndoDispatcher wiring is a tracked follow-up
//! (`docs/sessions/2026-05-24-tool-registry-gap.md` F5). When that
//! lands, the change envelope shape is `before = WarehouseMartSnapshot{
//! mart_name, ddl: prior }`, `after = WarehouseMartSnapshot{ mart_name,
//! ddl: None }`, op = `Op::Delete`. Until then drops are not
//! recoverable through `rubix.undo.last`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::warehouse::mart_drop::{WarehouseMartDropRequest, WarehouseMartDropResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};

use crate::warehouse::store::{WarehouseMartSnapshot, WarehouseWriter};

/// Concrete [`Tool`] for `rubix.warehouse.mart.drop`.
pub struct WarehouseMartDropTool {
    writer: Arc<dyn WarehouseWriter>,
}

impl WarehouseMartDropTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn WarehouseWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for WarehouseMartDropTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.mart.drop".to_owned(),
            description: rubix_spi::dto::warehouse::mart_drop::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mart_name": { "type": "string", "minLength": 1 }
                },
                "required": ["mart_name"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseMartDropRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseMartDropRequest: {e}"),
            })?;

        let prior = self.writer.show_create_mart(&req.mart_name).await?;
        let was_present = prior.is_some();
        // Restoring an absent snapshot is the established "drop"
        // path — see [`InMemoryWarehouseWriter::restore_mart`].
        self.writer
            .restore_mart(&WarehouseMartSnapshot {
                mart_name: req.mart_name.clone(),
                ddl: None,
            })
            .await?;
        let dropped_at_ms = now_epoch_ms();

        let code = if was_present {
            "rubix.warehouse.mart.dropped"
        } else {
            "rubix.warehouse.mart.absent"
        };
        let summary = Diagnostic::new(MessageKey::parse(code).expect("hard-coded key parses"))
            .with_param("mart", DiagnosticParam::String(req.mart_name.clone()));

        let response = WarehouseMartDropResponse {
            summary,
            mart_name: req.mart_name,
            was_present,
            dropped_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warehouse::store::InMemoryWarehouseWriter;

    #[tokio::test]
    async fn drop_of_absent_mart_is_no_op_with_absent_diagnostic() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        let tool = WarehouseMartDropTool::new(writer.clone());
        let out = tool
            .invoke(serde_json::json!({"mart_name": "ghost"}))
            .await
            .unwrap();
        let resp: WarehouseMartDropResponse = serde_json::from_value(out).unwrap();
        assert!(!resp.was_present);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.mart.absent");
        assert!(writer.mart("ghost").is_none());
    }

    #[tokio::test]
    async fn drop_of_present_mart_removes_it_with_dropped_diagnostic() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        writer.seed_mart("system_disk_history", "CREATE TABLE ...");
        let tool = WarehouseMartDropTool::new(writer.clone());
        let out = tool
            .invoke(serde_json::json!({"mart_name": "system_disk_history"}))
            .await
            .unwrap();
        let resp: WarehouseMartDropResponse = serde_json::from_value(out).unwrap();
        assert!(resp.was_present);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.mart.dropped");
        assert!(writer.mart("system_disk_history").is_none());
    }
}
