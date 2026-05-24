//! `rubix.clickhouse.mart.create` — tool dispatch.
//!
//! Snapshots the prior `SHOW CREATE TABLE` body via the shared
//! [`ChWriter`], runs the supplied `CREATE TABLE` DDL, and emits
//! either `rubix.clickhouse.mart.created` (happy path) or
//! `rubix.clickhouse.mart.already_exists` when the table was
//! already present (idempotent no-op, no Change recorded).
//!
//! Snapshot shape:
//!
//! - First create — `before = ChMartSnapshot { ddl: None }`,
//!   `after = ChMartSnapshot { ddl: Some(new) }`. Undo issues
//!   `DROP TABLE IF EXISTS` and recovers the schema but NOT the
//!   rows ingested between the create and the undo (documented
//!   in the design doc and surfaced in the diagnostic message).
//! - Re-create against an already-present mart — no Change is
//!   produced; the diagnostic carries the `already_exists` key.
//!
//! See
//! [docs/design/clickhouse-rules/](../../../../docs/design/clickhouse-rules/README.md).

use std::sync::Arc;

use async_trait::async_trait;
use rubix_spi::dto::clickhouse::mart_create::{
    ClickhouseMartCreateRequest, ClickhouseMartCreateResponse,
};
use serde_json::Value;
use starter_spi::authz::ResourceRef;
use starter_spi::changelog::Op;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_undo::ChangeDraft;

use crate::clickhouse::store::{ChMartSnapshot, ChWriter, CH_MART_KIND};
use crate::undo::dispatch::ReversibleTool;

/// Concrete [`Tool`] for `rubix.clickhouse.mart.create`.
pub struct ClickhouseMartCreateTool {
    writer: Arc<dyn ChWriter>,
}

impl ClickhouseMartCreateTool {
    /// Wrap the shared writer.
    pub fn new(writer: Arc<dyn ChWriter>) -> Self {
        Self { writer }
    }
}

#[async_trait]
impl Tool for ClickhouseMartCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.clickhouse.mart.create".to_owned(),
            description: rubix_spi::dto::clickhouse::mart_create::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "mart_name": { "type": "string", "minLength": 1 },
                    "ddl":       { "type": "string", "minLength": 1 }
                },
                "required": ["mart_name", "ddl"],
                "additionalProperties": false
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: ClickhouseMartCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("ClickhouseMartCreateRequest: {e}"),
            })?;
        validate_create_table(&req.ddl)?;

        let (prior, new) = self.writer.apply_mart_ddl(&req.mart_name, &req.ddl).await?;
        let was_already_present = prior.ddl.is_some();
        let created_at_ms = now_epoch_ms();

        let key = if was_already_present {
            "rubix.clickhouse.mart.already_exists"
        } else {
            "rubix.clickhouse.mart.created"
        };
        let summary = Diagnostic::new(
            MessageKey::parse(key).expect("hard-coded key parses"),
        )
        .with_param("mart", DiagnosticParam::String(req.mart_name.clone()))
        .with_param("at", DiagnosticParam::Timestamp(created_at_ms));

        let response = ClickhouseMartCreateResponse {
            summary,
            mart_name: req.mart_name,
            // `new.ddl` is `Some` either way; `prior.ddl` is what
            // matters for the snapshot — surface it on the wire.
            prior_ddl: prior.ddl,
            was_already_present,
            created_at_ms,
        };
        let _ = new; // documented above: we only need `prior` here.
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

impl ReversibleTool for ClickhouseMartCreateTool {
    fn change_for(&self, input: &Value, output: &Value) -> Option<ChangeDraft> {
        let req: ClickhouseMartCreateRequest = serde_json::from_value(input.clone()).ok()?;
        let resp: ClickhouseMartCreateResponse = serde_json::from_value(output.clone()).ok()?;
        if resp.was_already_present {
            // No state change — recording would let undo silently
            // drop a mart the caller did not actually create.
            return None;
        }
        let before = ChMartSnapshot {
            mart_name: resp.mart_name.clone(),
            ddl: None,
        };
        let after = ChMartSnapshot {
            mart_name: resp.mart_name.clone(),
            ddl: Some(req.ddl),
        };
        Some(ChangeDraft {
            resource: ResourceRef {
                kind: CH_MART_KIND.into(),
                id: Some(resp.mart_name),
                owner: None,
                tenant: None,
            },
            op: Op::Create,
            before: Some(serde_json::to_value(&before).ok()?),
            after: Some(serde_json::to_value(&after).ok()?),
            resource_version: None,
            correlation: None,
        })
    }
}

fn validate_create_table(ddl: &str) -> Result<()> {
    let trimmed = ddl.trim_start().to_ascii_uppercase();
    if trimmed.starts_with("CREATE TABLE") {
        return Ok(());
    }
    Err(Error::Invalid {
        message: format!(
            "rubix.clickhouse.mart.create: ddl must begin with CREATE TABLE (got: {:?})",
            ddl.chars().take(40).collect::<String>(),
        ),
    })
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

    #[tokio::test]
    async fn fresh_create_emits_created_and_records_create_with_empty_before() {
        let writer = Arc::new(InMemoryChWriter::new());
        let tool = ClickhouseMartCreateTool::new(writer.clone());
        let input = serde_json::json!({
            "mart_name": "system_disk_history",
            "ddl": "CREATE TABLE system_disk_history (ts DateTime) ENGINE = MergeTree ORDER BY ts",
        });
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: ClickhouseMartCreateResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(resp.summary.code.as_str(), "rubix.clickhouse.mart.created");
        assert!(!resp.was_already_present);
        assert!(resp.prior_ddl.is_none());

        let draft = tool.change_for(&input, &out).expect("draft present");
        assert!(matches!(draft.op, Op::Create));
        let before: ChMartSnapshot = serde_json::from_value(draft.before.unwrap()).unwrap();
        assert!(
            before.ddl.is_none(),
            "empty-prior snapshot drives the DROP TABLE inverse",
        );
    }

    #[tokio::test]
    async fn second_create_emits_already_exists_and_skips_draft() {
        let writer = Arc::new(InMemoryChWriter::new());
        writer.seed_mart("system_disk_history", "CREATE TABLE ...");
        let tool = ClickhouseMartCreateTool::new(writer);
        let input = serde_json::json!({
            "mart_name": "system_disk_history",
            "ddl": "CREATE TABLE system_disk_history (ts DateTime) ENGINE = MergeTree ORDER BY ts",
        });
        let out = tool.invoke(input.clone()).await.unwrap();
        let resp: ClickhouseMartCreateResponse = serde_json::from_value(out.clone()).unwrap();
        assert_eq!(
            resp.summary.code.as_str(),
            "rubix.clickhouse.mart.already_exists",
        );
        assert!(resp.was_already_present);
        assert!(
            tool.change_for(&input, &out).is_none(),
            "no-op create must not record a Change",
        );
    }

    #[tokio::test]
    async fn non_create_table_ddl_is_rejected() {
        let tool = ClickhouseMartCreateTool::new(Arc::new(InMemoryChWriter::new()));
        let err = tool
            .invoke(serde_json::json!({
                "mart_name": "x",
                "ddl": "ALTER TABLE x ADD COLUMN c Int32",
            }))
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Invalid { .. }));
    }
}
