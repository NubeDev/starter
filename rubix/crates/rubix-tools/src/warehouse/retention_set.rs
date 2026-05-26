//! `rubix.warehouse.retention.set` — tool dispatch.
//!
//! Probes the current TTL (via [`WarehouseWriter::current_retention`],
//! which the production impl backs with a `SELECT … FROM
//! system.tables` query), compares with the requested value, and
//! either runs the `ALTER TABLE … MODIFY TTL` and records a
//! Change, or short-circuits with `rubix.warehouse.retention.unchanged`
//! and skips the Change.
//!
//! Snapshot shape: `Op::Update`, `before = WarehouseRetentionSnapshot
//! { days: prior }`, `after = WarehouseRetentionSnapshot { days: new }`.
//! See
//! [docs/design/warehouse-rules/](../../../../docs/design/warehouse-rules/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::warehouse::retention_set::{
    WarehouseRetentionSetRequest, WarehouseRetentionSetResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::warehouse::store::{
    WarehouseRetentionSnapshot, WarehouseWriter, WAREHOUSE_RETENTION_KIND,
};

/// Concrete [`Tool`] for `rubix.warehouse.retention.set`.
pub struct WarehouseRetentionSetTool {
    writer: Arc<dyn WarehouseWriter>,
}

impl WarehouseRetentionSetTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn WarehouseWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for WarehouseRetentionSetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.retention.set".to_owned(),
            description: rubix_spi::dto::warehouse::retention_set::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "table_name": { "type": "string", "minLength": 1 },
                    "days":       { "type": "integer", "minimum": 0 }
                },
                "required": ["table_name", "days"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseRetentionSetRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseRetentionSetRequest: {e}"),
            })?;

        let prior_days = self.writer.current_retention(&req.table_name).await?;
        let requested = if req.days == 0 { None } else { Some(req.days) };
        let was_unchanged = prior_days == requested;
        let set_at_ms = now_epoch_ms();

        let (prior_snap, _new_snap) = if was_unchanged {
            // No DDL fired — synthesize the snapshot pair from the
            // probe so the response shape is uniform.
            let snap = WarehouseRetentionSnapshot {
                table_name: req.table_name.clone(),
                days: prior_days,
            };
            (snap.clone(), snap)
        } else {
            self.writer
                .apply_retention(&req.table_name, req.days)
                .await?
        };

        let key = if was_unchanged {
            "rubix.warehouse.retention.unchanged"
        } else {
            "rubix.warehouse.retention.set"
        };
        let summary = Diagnostic::new(MessageKey::parse(key).expect("hard-coded key parses"))
            .with_param("table", DiagnosticParam::String(req.table_name.clone()))
            .with_param("days", DiagnosticParam::I64(req.days as i64))
            .with_param("at", DiagnosticParam::Timestamp(set_at_ms));

        let response = WarehouseRetentionSetResponse {
            summary,
            table_name: req.table_name,
            prior_days: prior_snap.days,
            days: req.days,
            was_unchanged,
            set_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for WarehouseRetentionSetTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: WarehouseRetentionSetResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No-op — recording would let undo flip a value the
            // caller did not actually change.
            return None;
        }
        let before = WarehouseRetentionSnapshot {
            table_name: resp.table_name.clone(),
            days: resp.prior_days,
        };
        let after = WarehouseRetentionSnapshot {
            table_name: resp.table_name.clone(),
            days: if resp.days == 0 {
                None
            } else {
                Some(resp.days)
            },
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: WAREHOUSE_RETENTION_KIND.into(),
                id: Some(resp.table_name),
                owner: None,
                tenant: None,
            },
            serde_json::to_value(&before).ok()?,
            serde_json::to_value(&after).ok()?,
        ))
    }
}

fn now_epoch_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warehouse::store::InMemoryWarehouseWriter;
    use starter_spi::changelog::Op;

    #[tokio::test]
    async fn set_changes_ttl_and_emits_set_diagnostic() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        writer.seed_retention("system_disk_history", 90);
        let tool = WarehouseRetentionSetTool::new(writer.clone());
        let out = tool
            .invoke(serde_json::json!({"table_name": "system_disk_history", "days": 30}))
            .await
            .unwrap();
        let resp: WarehouseRetentionSetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.retention.set");
        assert_eq!(resp.prior_days, Some(90));
        assert_eq!(resp.days, 30);
        assert!(!resp.was_unchanged);
        assert_eq!(writer.retention("system_disk_history"), Some(30));
    }

    #[tokio::test]
    async fn matching_value_short_circuits_with_unchanged_and_no_draft() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        writer.seed_retention("system_disk_history", 30);
        let tool = WarehouseRetentionSetTool::new(writer);
        let input = serde_json::json!({"table_name": "system_disk_history", "days": 30});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: WarehouseRetentionSetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.warehouse.retention.unchanged",
        );
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "unchanged retention must not record a Change",
        );
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        writer.seed_retention("system_disk_history", 90);
        let tool = WarehouseRetentionSetTool::new(writer);
        let input = serde_json::json!({"table_name": "system_disk_history", "days": 30});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: WarehouseRetentionSnapshot =
            serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: WarehouseRetentionSnapshot =
            serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.days, Some(90));
        assert_eq!(after.days, Some(30));
    }
}
