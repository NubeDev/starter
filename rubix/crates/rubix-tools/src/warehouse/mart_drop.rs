//! `rubix.warehouse.mart.drop` — drop a mart (continuous aggregate).
//!
//! Idempotent: if the mart is absent the call is a no-op and
//! returns `rubix.warehouse.mart.absent`. Otherwise the cagg is
//! dropped (`DROP MATERIALIZED VIEW IF EXISTS ... CASCADE`) and
//! `rubix.warehouse.mart.dropped` is returned.
//!
//! Identifier safety: PostgreSQL identifiers cannot be parameter-
//! bound, so the mart name is interpolated. We restrict it to the
//! `[A-Za-z0-9_]+` charset to prevent injection. Anything outside
//! that set is rejected with `Error::Invalid`.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::mart_drop::{WarehouseMartDropRequest, WarehouseMartDropResponse};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::{cagg, WarehouseClient};

use crate::insights::store::now_epoch_ms;

pub struct WarehouseMartDropTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseMartDropTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseMartDropTool").finish()
    }
}

impl WarehouseMartDropTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

pub(crate) fn validate_identifier(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::Invalid {
            message: "identifier must not be blank".to_owned(),
        });
    }
    let ok = name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !ok {
        return Err(Error::Invalid {
            message: format!("identifier {name:?} contains characters outside [A-Za-z0-9_]"),
        });
    }
    Ok(())
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
        validate_identifier(&req.mart_name)?;

        let was_present = cagg::view_snapshot(&self.client, &req.mart_name)
            .await
            .ok()
            .flatten()
            .is_some();

        if was_present {
            let stmt = format!("DROP MATERIALIZED VIEW IF EXISTS {} CASCADE", req.mart_name);
            sqlx::query(&stmt)
                .execute(self.client.pool())
                .await
                .map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;
        }

        let dropped_at_ms = now_epoch_ms();
        let code = if was_present {
            "rubix.warehouse.mart.dropped"
        } else {
            "rubix.warehouse.mart.absent"
        };
        let summary = Diagnostic::new(MessageKey::parse(code).expect("hard-coded key parses"))
            .with_param("mart", DiagnosticParam::String(req.mart_name.clone()))
            .with_param("at", DiagnosticParam::I64(dropped_at_ms));

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifier_allows_alnum_and_underscore() {
        assert!(validate_identifier("samples_1h_mart").is_ok());
        assert!(validate_identifier("Mart42").is_ok());
    }

    #[test]
    fn identifier_rejects_quotes_and_semicolons() {
        assert!(validate_identifier("samples; DROP TABLE x").is_err());
        assert!(validate_identifier("\"samples\"").is_err());
        assert!(validate_identifier("").is_err());
    }
}
