//! `rubix.clickhouse.retention.set` — tool dispatch.
//!
//! Probes the current TTL (via [`ChWriter::current_retention`],
//! which the production impl backs with a `SELECT … FROM
//! system.tables` query), compares with the requested value, and
//! either runs the `ALTER TABLE … MODIFY TTL` and records a
//! Change, or short-circuits with `rubix.clickhouse.retention.unchanged`
//! and skips the Change.
//!
//! Snapshot shape: `Op::Update`, `before = ChRetentionSnapshot
//! { days: prior }`, `after = ChRetentionSnapshot { days: new }`.
//! See
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::retention_set::{
    ClickhouseRetentionSetRequest, ClickhouseRetentionSetResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::clickhouse::store::{ChRetentionSnapshot, ChWriter, CH_RETENTION_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.clickhouse.retention.set`.
pub struct ClickhouseRetentionSetTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseRetentionSetTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseRetentionSetTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.retention.set".to_owned(),
            description: rubix_spi::dto::clickhouse::retention_set::DESCRIPTOR
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
        let req: ClickhouseRetentionSetRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseRetentionSetRequest: {e}"),
            })?;

        let prior_days = self.writer.current_retention(&req.table_name).await?;
        let requested = if req.days == 0 { None } else { Some(req.days) };
        let was_unchanged = prior_days == requested;
        let set_at_ms = now_epoch_ms();

        let (prior_snap, _new_snap) = if was_unchanged {
            // No DDL fired — synthesize the snapshot pair from the
            // probe so the response shape is uniform.
            let snap = ChRetentionSnapshot {
                table_name: req.table_name.clone(),
                days: prior_days,
            };
            (snap.clone(), snap)
        } else {
            self.writer.apply_retention(&req.table_name, req.days).await?
        };

        let key = if was_unchanged {
            "rubix.clickhouse.retention.unchanged"
        } else {
            "rubix.clickhouse.retention.set"
        };
        let summary = Diagnostic::new(
            MessageKey::parse(key).expect("hard-coded key parses"),
        )
        .with_param("table", DiagnosticParam::String(req.table_name.clone()))
        .with_param("days", DiagnosticParam::I64(req.days as i64))
        .with_param("at", DiagnosticParam::Timestamp(set_at_ms));

        let response = ClickhouseRetentionSetResponse {
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

impl ReversibleTool for ClickhouseRetentionSetTool {
    fn change_for(&self, _input: &Value, output: &Value) -> Option<ChangeDraft> {
        let resp: ClickhouseRetentionSetResponse =
            serde_json::from_value(output.clone()).ok()?;
        if resp.was_unchanged {
            // No-op — recording would let undo flip a value the
            // caller did not actually change.
            return None;
        }
        let before = ChRetentionSnapshot {
            table_name: resp.table_name.clone(),
            days: resp.prior_days,
        };
        let after = ChRetentionSnapshot {
            table_name: resp.table_name.clone(),
            days: if resp.days == 0 { None } else { Some(resp.days) },
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: CH_RETENTION_KIND.into(),
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
    use crate::clickhouse::store::InMemoryChWriter;
    use starter_spi::changelog::Op;

    #[tokio::test]
    async fn set_changes_ttl_and_emits_set_diagnostic() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_retention("system_disk_history", 90);
        let tool = ClickhouseRetentionSetTool::new(writer.clone());
        let out = tool
            .invoke(serde_json::json!({"table_name": "system_disk_history", "days": 30}))
            .await
            .unwrap();
        let resp: ClickhouseRetentionSetResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.clickhouse.retention.set");
        assert_eq!(resp.prior_days, Some(90));
        assert_eq!(resp.days, 30);
        assert!(!resp.was_unchanged);
        assert_eq!(writer.retention("system_disk_history"), Some(30));
    }

    #[tokio::test]
    async fn matching_value_short_circuits_with_unchanged_and_no_draft() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_retention("system_disk_history", 30);
        let tool = ClickhouseRetentionSetTool::new(writer);
        let input = serde_json::json!({"table_name": "system_disk_history", "days": 30});
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: ClickhouseRetentionSetResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.clickhouse.retention.unchanged",
        );
        assert!(resp.was_unchanged);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "unchanged retention must not record a Change",
        );
    }

    #[tokio::test]
    async fn change_for_records_update_with_before_after_snapshots() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_retention("system_disk_history", 90);
        let tool = ClickhouseRetentionSetTool::new(writer);
        let input = serde_json::json!({"table_name": "system_disk_history", "days": 30});
        let out = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: ChRetentionSnapshot =
            serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: ChRetentionSnapshot =
            serde_json::from_value(draft.after.unwrap()).unwrap();
        assert_eq!(before.days, Some(90));
        assert_eq!(after.days, Some(30));
    }
}
