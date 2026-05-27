//! `rubix.warehouse.rule.write` — execute CREATE/ALTER DDL against
//! the warehouse and return the prior view definition.
//!
//! Validation is parse-only: the DDL must begin with `CREATE` or
//! `ALTER` (case-insensitive). `DROP` is refused per the DTO
//! contract — destructive removals go through `mart.drop` or
//! `undo.last`.
//!
//! Undo wiring: the response carries `prior_ddl` so callers can
//! snapshot it externally. The crate-internal `Reversible` /
//! `undo_snapshots` plumbing referenced by the DTO docstring does
//! not exist in source yet; see the parent proposal's deferred
//! Stage 2 work.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::rule_write::{
    WarehouseRuleWriteRequest, WarehouseRuleWriteResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::{cagg, WarehouseClient};

use crate::insights::store::now_epoch_ms;

pub struct WarehouseRuleWriteTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseRuleWriteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseRuleWriteTool").finish()
    }
}

impl WarehouseRuleWriteTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

/// Returns `Ok(())` when the DDL is a CREATE or ALTER statement.
/// `DROP` and anything else is refused with `rubix.warehouse.rule.invalid`.
fn validate_ddl(ddl: &str) -> Result<()> {
    let head = ddl.trim_start();
    let lower: String = head
        .chars()
        .take(8)
        .collect::<String>()
        .to_ascii_lowercase();
    if lower.starts_with("create ") || lower.starts_with("alter ") {
        Ok(())
    } else {
        Err(Error::Invalid {
            message: "rubix.warehouse.rule.invalid: DDL must begin with CREATE or ALTER".to_owned(),
        })
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
        if req.rule_name.trim().is_empty() {
            return Err(Error::Invalid {
                message: "WarehouseRuleWriteRequest: rule_name must not be blank".to_owned(),
            });
        }
        validate_ddl(&req.ddl)?;

        let prior_ddl = cagg::view_snapshot(&self.client, &req.rule_name)
            .await
            .ok()
            .flatten()
            .map(|s| s.view_definition);

        sqlx::query(&req.ddl)
            .execute(self.client.pool())
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?;

        let written_at_ms = now_epoch_ms();
        let summary = Diagnostic::new(
            MessageKey::parse("rubix.warehouse.rule.written").expect("hard-coded key parses"),
        )
        .with_param("rule", DiagnosticParam::String(req.rule_name.clone()))
        .with_param("at", DiagnosticParam::I64(written_at_ms));

        let response = WarehouseRuleWriteResponse {
            summary,
            rule_name: req.rule_name,
            prior_ddl,
            written_at_ms,
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
    fn validate_accepts_create_and_alter() {
        assert!(validate_ddl("CREATE MATERIALIZED VIEW v AS SELECT 1").is_ok());
        assert!(validate_ddl("  alter view v RENAME TO w").is_ok());
    }

    #[test]
    fn validate_rejects_drop_and_other() {
        assert!(validate_ddl("DROP VIEW v").is_err());
        assert!(validate_ddl("SELECT 1").is_err());
        assert!(validate_ddl("").is_err());
    }
}
