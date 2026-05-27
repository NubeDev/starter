//! `rubix.warehouse.retention.set` — set or clear a retention
//! policy on a hypertable.
//!
//! Snapshots the prior `drop_after` value from
//! `timescaledb_information.jobs`, then either:
//!
//! - `days == 0`  → [`retention::remove_retention_policy`]
//! - `days  > 0`  → [`retention::add_retention_policy`]
//!
//! When the requested value already matches the current value the
//! call is a no-op and emits `rubix.warehouse.retention.unchanged`.

use async_trait::async_trait;
use rubix_spi::dto::warehouse::retention_set::{
    WarehouseRetentionSetRequest, WarehouseRetentionSetResponse,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::{retention, WarehouseClient};

use crate::insights::store::now_epoch_ms;
use crate::warehouse::mart_drop::validate_identifier;

pub struct WarehouseRetentionSetTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseRetentionSetTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseRetentionSetTool").finish()
    }
}

impl WarehouseRetentionSetTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
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
        validate_identifier(&req.table_name)?;

        let prior_days = retention::snapshot_days(&self.client, &req.table_name)
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?
            .and_then(|d| u32::try_from(d).ok());

        let was_unchanged = prior_days == Some(req.days) || (prior_days.is_none() && req.days == 0);

        if !was_unchanged {
            if req.days == 0 {
                retention::remove_retention_policy(&self.client, &req.table_name)
                    .await
                    .map_err(|e| Error::Internal {
                        source: Box::new(e),
                    })?;
            } else {
                let days_i32 = i32::try_from(req.days).map_err(|_| Error::Invalid {
                    message: format!("days {} exceeds i32::MAX", req.days),
                })?;
                retention::add_retention_policy(&self.client, &req.table_name, days_i32)
                    .await
                    .map_err(|e| Error::Internal {
                        source: Box::new(e),
                    })?;
            }
        }

        let set_at_ms = now_epoch_ms();
        let code = if was_unchanged {
            "rubix.warehouse.retention.unchanged"
        } else {
            "rubix.warehouse.retention.set"
        };
        let summary = Diagnostic::new(MessageKey::parse(code).expect("hard-coded key parses"))
            .with_param("table", DiagnosticParam::String(req.table_name.clone()))
            .with_param("days", DiagnosticParam::I64(req.days as i64))
            .with_param("at", DiagnosticParam::I64(set_at_ms));

        let response = WarehouseRetentionSetResponse {
            summary,
            table_name: req.table_name,
            prior_days,
            days: req.days,
            was_unchanged,
            set_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}
