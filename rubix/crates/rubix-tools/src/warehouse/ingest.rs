//! `rubix.warehouse.ingest` — append synth meter readings into the
//! Timescale `samples` hypertable.
//!
//! Input shape is the `SynthEmitResponse` (`{ rows: [...], stats }`),
//! which is exactly what the producer flow's link
//! `synth.output -> ingest.input` projects. Unknown / extra fields
//! are ignored so the tool stays forward-compatible if synth grows
//! new output fields.

use async_trait::async_trait;
use rubix_spi::dto::dataflow::synth::{MeterReading, ReadingQuality};
use serde::Deserialize;
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_warehouse::WarehouseClient;
use tracing::warn;

/// Concrete `Tool` impl for `rubix.warehouse.ingest`.
pub struct WarehouseIngestTool {
    client: WarehouseClient,
}

impl std::fmt::Debug for WarehouseIngestTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseIngestTool").finish()
    }
}

impl WarehouseIngestTool {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }
}

/// Wire-level input. Accepts the synth response shape directly so
/// the producer flow's `synth.output -> ingest.input` link Just Works.
#[derive(Debug, Deserialize)]
struct IngestRequest {
    #[serde(default)]
    rows: Vec<MeterReading>,
}

#[async_trait]
impl Tool for WarehouseIngestTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.ingest".to_owned(),
            description: "Append synth meter readings into the Timescale `samples` hypertable."
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "rows": {
                        "type": "array",
                        "items": { "type": "object" }
                    }
                }
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: IngestRequest = serde_json::from_value(input).map_err(|e| Error::Invalid {
            message: format!("IngestRequest: {e}"),
        })?;

        let mut inserted = 0usize;
        let mut skipped_nan = 0usize;

        for row in &req.rows {
            // NaN is not storable in `double precision`; the synth
            // tool flags suspect readings via `quality` but emits
            // f64::NAN as the value. Skip those (the cleaner verb
            // would normally do this; we inline it).
            if row.value.is_nan() {
                skipped_nan += 1;
                continue;
            }
            // Map quality enum to the SMALLINT column (0 = ok, 1 =
            // suspect, 2 = missing).
            let quality: i16 = match row.quality {
                ReadingQuality::Ok => 0,
                ReadingQuality::Suspect => 1,
                ReadingQuality::Missing => 2,
            };
            let tags = serde_json::json!({
                "kind": row.kind,
                "unit": row.unit,
            });
            let ts_ms = row.epoch_ms;
            let result = sqlx::query(
                "INSERT INTO samples (tenant_id, entity_id, ts, value_num, value_str, value_bool, quality, tags) \
                 VALUES ($1, $2, to_timestamp($3::double precision / 1000.0), $4, NULL, NULL, $5, $6)",
            )
            .bind(&row.tenant_id)
            .bind(&row.meter_id)
            .bind(ts_ms as f64)
            .bind(row.value)
            .bind(quality)
            .bind(&tags)
            .execute(self.client.pool())
            .await;
            match result {
                Ok(_) => inserted += 1,
                Err(e) => {
                    warn!(
                        target: "rubix.warehouse.ingest",
                        meter_id = %row.meter_id,
                        error = %e,
                        "sample insert failed",
                    );
                }
            }
        }

        Ok(serde_json::json!({
            "inserted": inserted,
            "skipped_nan": skipped_nan,
        }))
    }
}
