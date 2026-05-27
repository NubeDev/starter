//! `rubix.warehouse.mart.create` — provision a mart (continuous
//! aggregate or table) from a CREATE statement.
//!
//! Idempotent: if a continuous aggregate with the same name already
//! exists the verb is a no-op and returns
//! `rubix.warehouse.mart.already_exists`. Otherwise the DDL runs
//! and `rubix.warehouse.mart.created` is returned.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::mart_create::{
    WarehouseMartCreateRequest, WarehouseMartCreateResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::{cagg, WarehouseClient};

use crate::insights::store::now_epoch_ms;

pub struct WarehouseMartCreateTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseMartCreateTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseMartCreateTool").finish()
    }
}

impl WarehouseMartCreateTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

fn validate_ddl(ddl: &str) -> Result<()> {
    let head = ddl.trim_start();
    let lower = head
        .chars()
        .take(7)
        .collect::<String>()
        .to_ascii_lowercase();
    if lower.starts_with("create ") {
        Ok(())
    } else {
        Err(Error::Invalid {
            message: "rubix.warehouse.mart.invalid: DDL must begin with CREATE".to_owned(),
        })
    }
}

#[async_trait]
impl Tool for WarehouseMartCreateTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.mart.create".to_owned(),
            description: rubix_spi::dto::warehouse::mart_create::DESCRIPTOR
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
        let req: WarehouseMartCreateRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseMartCreateRequest: {e}"),
            })?;
        if req.mart_name.trim().is_empty() {
            return Err(Error::Invalid {
                message: "WarehouseMartCreateRequest: mart_name must not be blank".to_owned(),
            });
        }
        validate_ddl(&req.ddl)?;

        let prior = cagg::view_snapshot(&self.client, &req.mart_name)
            .await
            .ok()
            .flatten();

        let created_at_ms = now_epoch_ms();
        let (code, was_already_present, prior_ddl) = if let Some(snap) = prior {
            (
                "rubix.warehouse.mart.already_exists",
                true,
                Some(snap.view_definition),
            )
        } else {
            sqlx::query(&req.ddl)
                .execute(self.client.pool())
                .await
                .map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;
            ("rubix.warehouse.mart.created", false, None)
        };

        let summary = Diagnostic::new(MessageKey::parse(code).expect("hard-coded key parses"))
            .with_param("mart", DiagnosticParam::String(req.mart_name.clone()))
            .with_param("at", DiagnosticParam::I64(created_at_ms));

        let response = WarehouseMartCreateResponse {
            summary,
            mart_name: req.mart_name,
            prior_ddl,
            was_already_present,
            created_at_ms,
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
    fn validate_accepts_create() {
        assert!(validate_ddl("CREATE MATERIALIZED VIEW v AS SELECT 1").is_ok());
    }

    #[test]
    fn validate_rejects_alter_and_drop() {
        assert!(validate_ddl("ALTER VIEW v RENAME TO w").is_err());
        assert!(validate_ddl("DROP VIEW v").is_err());
    }
}
