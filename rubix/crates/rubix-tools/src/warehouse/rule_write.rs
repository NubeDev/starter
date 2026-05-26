//! `rubix.warehouse.rule.write` — tool dispatch.
//!
//! Snapshots the prior `SHOW CREATE TABLE` body via the shared
//! [`WarehouseWriter`], runs the supplied DDL, and emits a `Diagnostic`
//! keyed `rubix.warehouse.rule.written`. The companion
//! `change_for` impl produces an `Op::Update` [`ChangeDraft`]
//! carrying `before = WarehouseRuleSnapshot(prior_ddl)` and
//! `after = WarehouseRuleSnapshot(new_ddl)` so the undo dispatcher walks
//! the write back through [`super::store::WarehouseRuleReversible`].
//!
//! Invalid input (DDL that does not begin with `CREATE` or
//! `ALTER`) yields `rubix.warehouse.rule.invalid` *before* the
//! writer is called — no snapshot row is created for a refused
//! write. See
//! [docs/design/warehouse-rules/](../../../../docs/design/warehouse-rules/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::warehouse::rule_write::{
    WarehouseRuleWriteRequest, WarehouseRuleWriteResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::undo::dispatch::ReversibleTool;
use crate::warehouse::store::{WarehouseRuleSnapshot, WarehouseWriter, WAREHOUSE_RULE_KIND};

/// Concrete [`Tool`] for `rubix.warehouse.rule.write`.
pub struct WarehouseRuleWriteTool {
    writer: Arc<dyn WarehouseWriter>,
}

impl WarehouseRuleWriteTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn WarehouseWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for WarehouseRuleWriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.rule.write".to_owned(),
            description: rubix_spi::dto::warehouse::rule_write::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "rule_name": { "type": "string", "minLength": 1 },
                    "ddl":       { "type": "string", "minLength": 1 }
                },
                "required": ["rule_name", "ddl"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseRuleWriteRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseRuleWriteRequest: {e}"),
            })?;
        validate_ddl(&req.ddl)?;

        let (prior, _new) = self.writer.apply_rule_ddl(&req.rule_name, &req.ddl).await?;
        let written_at_ms = now_epoch_ms();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.warehouse.rule.written").expect("hard-coded key parses"),
        )
        .with_param("rule", DiagnosticParam::String(req.rule_name.clone()))
        .with_param("at", DiagnosticParam::Timestamp(written_at_ms));

        let response = WarehouseRuleWriteResponse {
            summary,
            rule_name: req.rule_name,
            prior_ddl: prior.ddl,
            written_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for WarehouseRuleWriteTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: WarehouseRuleWriteRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: WarehouseRuleWriteResponse = serde_json::from_value(output.clone()).ok()?;
        let before = WarehouseRuleSnapshot {
            rule_name: resp.rule_name.clone(),
            ddl: resp.prior_ddl,
        };
        let after = WarehouseRuleSnapshot {
            rule_name: resp.rule_name.clone(),
            ddl: Some(req.ddl),
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: WAREHOUSE_RULE_KIND.into(),
                id: Some(resp.rule_name),
                owner: None,
                tenant: None,
            },
            serde_json::to_value(&before).ok()?,
            serde_json::to_value(&after).ok()?,
        ))
    }
}

/// Refuse anything that is not a `CREATE` or `ALTER` — DROP goes
/// through the inverse-op path of a prior write, never as a direct
/// verb call.
fn validate_ddl(ddl: &str) -> Result<()> {
    let trimmed = ddl.trim_start().to_ascii_uppercase();
    if trimmed.starts_with("CREATE ") || trimmed.starts_with("ALTER ") {
        return Ok(());
    }
    Err(Error::Invalid {
        message: invalid_message(ddl),
    })
}

fn invalid_message(ddl: &str) -> String {
    let preview: String = ddl.chars().take(40).collect();
    let key = MessageKey::parse("rubix.warehouse.rule.invalid").expect("hard-coded key parses");
    format!(
        "{}: ddl must start with CREATE or ALTER (got: {preview:?})",
        key.as_str()
    )
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
    async fn write_emits_written_diagnostic_and_records_prior_ddl() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        writer.seed_mart("ignored", "x"); // unrelated state
        let tool = WarehouseRuleWriteTool::new(writer.clone());

        // First call: no prior DDL.
        let out = tool
            .invoke(serde_json::json!({
                "rule_name": "system_disk_rollup_1h",
                "ddl": "CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 1",
            }))
            .await
            .unwrap();
        let resp: WarehouseRuleWriteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.rule.written");
        assert!(resp.prior_ddl.is_none());

        // Second call: prior DDL is the body we just wrote.
        let out = tool
            .invoke(serde_json::json!({
                "rule_name": "system_disk_rollup_1h",
                "ddl": "CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 2",
            }))
            .await
            .unwrap();
        let resp: WarehouseRuleWriteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.prior_ddl.as_deref(),
            Some("CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 1"),
        );
    }

    #[tokio::test]
    async fn ddl_without_create_or_alter_is_rejected() {
        let tool = WarehouseRuleWriteTool::new(Arc::new(InMemoryWarehouseWriter::new()));
        let err = tool
            .invoke(serde_json::json!({
                "rule_name": "x",
                "ddl": "DROP TABLE x",
            }))
            .await
            .unwrap_err();
        let msg = match err {
            Error::Invalid { message } => message,
            other => panic!("expected Invalid, got {other:?}"),
        };
        assert!(msg.contains("rubix.warehouse.rule.invalid"), "msg: {msg}");
    }

    #[tokio::test]
    async fn change_for_returns_update_with_both_snapshots() {
        let writer = Arc::new(InMemoryWarehouseWriter::new());
        let tool = WarehouseRuleWriteTool::new(writer);
        let input = serde_json::json!({
            "rule_name": "r1",
            "ddl": "CREATE MATERIALIZED VIEW r1 AS SELECT 1",
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: WarehouseRuleSnapshot = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: WarehouseRuleSnapshot = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(before.ddl.is_none());
        assert!(after.ddl.is_some());
    }
}
