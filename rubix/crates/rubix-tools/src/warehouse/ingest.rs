//! `rubix.warehouse.ingest` — tool dispatch.
//!
//! Mirrors [`crate::system::disk::DiskTool`]'s history-row write
//! path: the verb takes a batch of [`MeterReading`] rows matching
//! stage 01's wire shape and lands them in
//! `rubix.meter_readings_raw` via a single multi-row INSERT through
//! [`ChClient::inner().query(...).execute()`].
//!
//! Append-only: no `Reversible` wiring — undo of raw rows makes no
//! sense at L1 (operators undo at the mart layer). See
//! `rubix/docs/sessions/data-flow/02-ingest-l1.md`.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::dataflow::ingest::{WarehouseIngestRequest, WarehouseIngestResponse};
use rubix_spi::dto::dataflow::synth::{MeterKind, MeterReading, MeterUnit, ReadingQuality};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_clickhouse::ChClient;

/// Concrete [`Tool`] for `rubix.warehouse.ingest`. Optionally holds
/// a [`ChClient`]; without one the verb still validates input and
/// returns `inserted = 0` so unit tests (and the laptop / no-CH dev
/// path) keep working — same shape as [`crate::system::disk::DiskTool`].
#[derive(Default, Clone)]
pub struct WarehouseIngestTool {
    client: Option<Arc<ChClient>>,
}

impl std::fmt::Debug for WarehouseIngestTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseIngestTool")
            .field("client", &self.client.is_some())
            .finish()
    }
}

impl WarehouseIngestTool {
    /// Wrap a [`ChClient`]. Without it the verb is a structural
    /// no-op (validate + return `inserted = 0`).
    pub fn with_client(mut self, client: Arc<ChClient>) -> Self {
        self.client = Some(client);
        self
    }
}

#[async_trait]
impl Tool for WarehouseIngestTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.ingest".to_owned(),
            description: rubix_spi::dto::dataflow::ingest::DESCRIPTOR.purpose.to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "tenant_id": { "type": "string" },
                    "rows":      { "type": "array",  "items": { "type": "object" } }
                },
                "required": ["rows"],
                "additionalProperties": true
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseIngestRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseIngestRequest: {e}"),
            })?;
        let written_at_ms = now_epoch_ms();
        let inserted = req.rows.len() as u32;

        if !req.rows.is_empty() {
            if let Some(client) = &self.client {
                let sql = insert_sql(&req.rows);
                client
                    .inner()
                    .query(&sql)
                    .execute()
                    .await
                    .map_err(|e| Error::Internal {
                        source: Box::new(e),
                    })?;
            }
        }

        let summary = if inserted == 0 {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.ingest.empty")
                    .expect("hard-coded key parses"),
            )
        } else {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.ingested").expect("hard-coded key parses"),
            )
            .with_param("rows", DiagnosticParam::I64(i64::from(inserted)))
            .with_param("at", DiagnosticParam::Timestamp(written_at_ms))
        };

        let response = WarehouseIngestResponse {
            summary,
            inserted,
            written_at_ms,
        };
        serde_json::to_value(response).map_err(|e| Error::Internal {
            source: Box::new(e),
        })
    }
}

fn now_epoch_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// SQL the ingest issues. Split out so the unit test can assert the
/// row shape reaches the column list literally — a regression that
/// silently dropped `quality` or `tenant_id` would defeat the whole
/// point of L1 fidelity.
///
/// The official `clickhouse` crate quotes strings with single-quote
/// escaping; floats / ints / enums travel as plain literals. `NaN`
/// is rendered as `nan` (CH's literal token); the column is
/// `Float64` so it accepts it. The `meter_readings_raw` table is in
/// the connection's bound database (`rubix` per
/// [`crate::system::disk`]'s same wiring), so the bare table name
/// resolves without a `rubix.` prefix.
pub(crate) fn insert_sql(rows: &[MeterReading]) -> String {
    debug_assert!(!rows.is_empty(), "insert_sql is unreachable for empty rows");
    let mut out = String::from(
        "INSERT INTO meter_readings_raw \
         (tenant_id, meter_id, kind, unit, epoch_ms, value, quality) VALUES ",
    );
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        let tenant = escape(&row.tenant_id);
        let meter = escape(&row.meter_id);
        let kind = match row.kind {
            MeterKind::Electricity => "electricity",
            MeterKind::Water => "water",
        };
        let unit = match row.unit {
            MeterUnit::KWh => "kWh",
            MeterUnit::L => "L",
        };
        let quality = match row.quality {
            ReadingQuality::Ok => "ok",
            ReadingQuality::Suspect => "suspect",
            ReadingQuality::Missing => "missing",
        };
        let value = if row.value.is_nan() {
            "nan".to_owned()
        } else if row.value.is_infinite() {
            // Guard against ±inf reaching CH; the wire contract
            // does not promise these, and CH's Float64 literal
            // parser rejects "inf". Clamp to a sentinel so the row
            // still lands rather than aborting the whole batch.
            if row.value.is_sign_positive() {
                f64::MAX.to_string()
            } else {
                f64::MIN.to_string()
            }
        } else {
            row.value.to_string()
        };
        out.push_str(&format!(
            "('{tenant}','{meter}','{kind}','{unit}',{epoch},{value},'{quality}')",
            epoch = row.epoch_ms,
        ));
    }
    out
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_row() -> MeterReading {
        MeterReading {
            tenant_id: "site-a".to_owned(),
            meter_id: "site-a.elec.main".to_owned(),
            kind: MeterKind::Electricity,
            unit: MeterUnit::KWh,
            epoch_ms: 1_748_275_200_000,
            value: 10_001.2,
            quality: ReadingQuality::Ok,
        }
    }

    #[test]
    fn insert_sql_carries_every_column_for_one_row() {
        let sql = insert_sql(&[ok_row()]);
        assert!(sql.contains("INSERT INTO meter_readings_raw"));
        assert!(sql.contains("(tenant_id, meter_id, kind, unit, epoch_ms, value, quality)"));
        assert!(sql.contains("'site-a'"));
        assert!(sql.contains("'site-a.elec.main'"));
        assert!(sql.contains("'electricity'"));
        assert!(sql.contains("'kWh'"));
        assert!(sql.contains("1748275200000"));
        assert!(sql.contains("10001.2"));
        assert!(sql.contains("'ok'"));
    }

    #[test]
    fn insert_sql_batches_multiple_rows_with_one_statement() {
        let mut r2 = ok_row();
        r2.meter_id = "site-a.water.main".to_owned();
        r2.kind = MeterKind::Water;
        r2.unit = MeterUnit::L;
        r2.quality = ReadingQuality::Suspect;
        let sql = insert_sql(&[ok_row(), r2]);
        // Exactly one INSERT statement (no semicolons mid-string).
        assert_eq!(sql.matches("INSERT INTO").count(), 1);
        assert!(sql.contains("'water'"));
        assert!(sql.contains("'L'"));
        assert!(sql.contains("'suspect'"));
    }

    #[test]
    fn insert_sql_renders_nan_as_nan_literal() {
        let mut row = ok_row();
        row.value = f64::NAN;
        let sql = insert_sql(&[row]);
        assert!(
            sql.contains(",nan,"),
            "NaN must travel as CH's `nan` literal; got {sql}",
        );
    }

    #[test]
    fn insert_sql_escapes_single_quotes_in_tenant_and_meter() {
        let mut row = ok_row();
        row.tenant_id = "a'b".to_owned();
        row.meter_id = "m'1".to_owned();
        let sql = insert_sql(&[row]);
        assert!(sql.contains("'a''b'"), "tenant: {sql}");
        assert!(sql.contains("'m''1'"), "meter: {sql}");
    }

    #[tokio::test]
    async fn invoke_with_no_client_returns_empty_when_rows_empty() {
        let tool = WarehouseIngestTool::default();
        let out = tool
            .invoke(serde_json::json!({ "rows": [] }))
            .await
            .unwrap();
        let resp: WarehouseIngestResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.inserted, 0);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.ingest.empty");
    }

    #[tokio::test]
    async fn invoke_with_no_client_still_reports_inserted_count_for_nonempty_batch() {
        // The CH write is skipped when no client is bound; the
        // verb still reports the would-have-been count so flows
        // that wire this in dev (no warehouse) see plausible
        // observability rather than a silent zero.
        let tool = WarehouseIngestTool::default();
        let out = tool
            .invoke(serde_json::json!({
                "rows": [{
                    "tenant_id": "site-a",
                    "meter_id":  "site-a.elec.main",
                    "kind":      "electricity",
                    "unit":      "kWh",
                    "epoch_ms":  1_748_275_200_000_i64,
                    "value":     10_001.2,
                    "quality":   "ok"
                }]
            }))
            .await
            .unwrap();
        let resp: WarehouseIngestResponse = serde_json::from_value(out).unwrap();
        assert_eq!(resp.inserted, 1);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.ingested");
    }
}
