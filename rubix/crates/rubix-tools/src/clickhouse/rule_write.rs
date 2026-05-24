//! `rubix.clickhouse.rule.write` — tool dispatch.
//!
//! Snapshots the prior `SHOW CREATE TABLE` body via the shared
//! [`ChWriter`], runs the supplied DDL, and emits a `Diagnostic`
//! keyed `rubix.clickhouse.rule.written`. The companion
//! `change_for` impl produces an `Op::Update` [`ChangeDraft`]
//! carrying `before = ChRuleSnapshot(prior_ddl)` and
//! `after = ChRuleSnapshot(new_ddl)` so the undo dispatcher walks
//! the write back through [`super::store::ChRuleReversible`].
//!
//! Invalid input (DDL that does not begin with `CREATE` or
//! `ALTER`) yields `rubix.clickhouse.rule.invalid` *before* the
//! writer is called — no snapshot row is created for a refused
//! write. See
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::rule_write::{
    ClickhouseRuleWriteRequest, ClickhouseRuleWriteResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::clickhouse::store::{ChRuleSnapshot, ChWriter, CH_RULE_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.clickhouse.rule.write`.
pub struct ClickhouseRuleWriteTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseRuleWriteTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseRuleWriteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.rule.write".to_owned(),
            description: rubix_spi::dto::clickhouse::rule_write::DESCRIPTOR
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
        let req: ClickhouseRuleWriteRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseRuleWriteRequest: {e}"),
            })?;
        validate_ddl(&req.ddl)?;

        let (prior, _new) = self.writer.apply_rule_ddl(&req.rule_name, &req.ddl).await?;
        let written_at_ms = now_epoch_ms();

        let summary = Diagnostic::new(
            MessageKey::parse("rubix.clickhouse.rule.written")
                .expect("hard-coded key parses"),
        )
        .with_param("rule", DiagnosticParam::String(req.rule_name.clone()))
        .with_param("at", DiagnosticParam::Timestamp(written_at_ms));

        let response = ClickhouseRuleWriteResponse {
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

impl ReversibleTool for ClickhouseRuleWriteTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: ClickhouseRuleWriteRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: ClickhouseRuleWriteResponse = serde_json::from_value(output.clone()).ok()?;
        let before = ChRuleSnapshot {
            rule_name: resp.rule_name.clone(),
            ddl: resp.prior_ddl,
        };
        let after = ChRuleSnapshot {
            rule_name: resp.rule_name.clone(),
            ddl: Some(req.ddl),
        };
        Some(ChangeDraft::update(
            ResourceRef {
                kind: CH_RULE_KIND.into(),
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
    let key = MessageKey::parse("rubix.clickhouse.rule.invalid")
        .expect("hard-coded key parses");
    format!("{}: ddl must start with CREATE or ALTER (got: {preview:?})", key.as_str())
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
    async fn write_emits_written_diagnostic_and_records_prior_ddl() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_mart("ignored", "x"); // unrelated state
        let tool = ClickhouseRuleWriteTool::new(writer.clone());

        // First call: no prior DDL.
        let out = tool
            .invoke(serde_json::json!({
                "rule_name": "system_disk_rollup_1h",
                "ddl": "CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 1",
            }))
            .await
            .unwrap();
        let resp: ClickhouseRuleWriteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.clickhouse.rule.written");
        assert!(resp.prior_ddl.is_none());

        // Second call: prior DDL is the body we just wrote.
        let out = tool
            .invoke(serde_json::json!({
                "rule_name": "system_disk_rollup_1h",
                "ddl": "CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 2",
            }))
            .await
            .unwrap();
        let resp: ClickhouseRuleWriteResponse = serde_json::from_value(out).unwrap();
        assert_eq!(
            resp.prior_ddl.as_deref(),
            Some("CREATE MATERIALIZED VIEW system_disk_rollup_1h AS SELECT 1"),
        );
    }

    #[tokio::test]
    async fn ddl_without_create_or_alter_is_rejected() {
        let tool = ClickhouseRuleWriteTool::new(Arc::new(InMemoryChWriter::new()));
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
        assert!(msg.contains("rubix.clickhouse.rule.invalid"), "msg: {msg}");
    }

    #[tokio::test]
    async fn change_for_returns_update_with_both_snapshots() {
        let writer = Arc::new(InMemoryChWriter::new());
        let tool = ClickhouseRuleWriteTool::new(writer);
        let input = serde_json::json!({
            "rule_name": "r1",
            "ddl": "CREATE MATERIALIZED VIEW r1 AS SELECT 1",
        });
        let output = tool.invoke(input.clone()).await.unwrap();
        let draft = tool.change_for(&input, &output).expect("draft present");
        assert!(matches!(draft.op, Op::Update));
        let before: ChRuleSnapshot = serde_json::from_value(draft.before.unwrap()).unwrap();
        let after: ChRuleSnapshot = serde_json::from_value(draft.after.unwrap()).unwrap();
        assert!(before.ddl.is_none());
        assert!(after.ddl.is_some());
    }
}
