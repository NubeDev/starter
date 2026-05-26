//! `rubix.warehouse.clean_minute` — tool dispatch.
//!
//! One verb call = one cleaner pass: run a single CH `INSERT INTO
//! rubix.meter_readings_1m SELECT ...` over the lookback window
//! ending at the previous complete minute. Idempotent because L2
//! is `ReplacingMergeTree` ordered by
//! `(tenant_id, meter_id, bucket_start)` — repeated passes
//! refine the same bucket rather than duplicating it.
//!
//! Cleaning rules (locked in stage 03 doc, §"What clean means here"):
//!
//! - **gap**     — bucket has no L1 row → emit `value = NULL`,
//!                 `quality = 'missing'`. Calendar synthesised
//!                 here so dashboards do not need to outer-join.
//! - **nan**     — bucket has L1 rows but every value is NaN →
//!                 `value = NULL`, `quality = 'nan'`.
//! - **clipped** — bucket's clean avg > 10 × 15-minute rolling
//!                 median for the same meter → `value = median`,
//!                 `quality = 'clipped'`. Stage doc bar #4 fails
//!                 the stage if any `quality='ok'` row breaks this.
//! - **ok**      — everything else.
//!
//! The "stuck" rule from the stage doc is intentionally deferred:
//! the producer's claim cadence is 60 s × 3 meters, so a 5-buckets-
//! of-the-same-value detection needs more rows than a typical
//! 5-minute lookback window emits. Track it as a follow-up.
//!
//! Append-only at the row level (no reversible wiring); operators
//! recover by re-running the cleaner over a wider lookback or by
//! dropping the L2 mart and rebuilding.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::dataflow::clean_minute::{
    WarehouseCleanMinuteRequest, WarehouseCleanMinuteResponse, DEFAULT_LOOKBACK_MINUTES,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_clickhouse::clickhouse;
use starter_store_clickhouse::clickhouse::Row;
use starter_store_clickhouse::ChClient;

/// Hard upper bound on the lookback knob. The cleaner runs every
/// minute; a window wider than 60 minutes would re-clean the same
/// buckets dozens of times per hour with no new information.
const MAX_LOOKBACK_MINUTES: u32 = 60;

/// Concrete [`Tool`] for `rubix.warehouse.clean_minute`. Optionally
/// holds a [`ChClient`]; without one the verb still validates input
/// and returns `rows = 0` so unit tests and the no-CH laptop dev
/// path keep working.
#[derive(Default, Clone)]
pub struct WarehouseCleanMinuteTool {
    client: Option<Arc<ChClient>>,
}

impl std::fmt::Debug for WarehouseCleanMinuteTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseCleanMinuteTool")
            .field("client", &self.client.is_some())
            .finish()
    }
}

impl WarehouseCleanMinuteTool {
    /// Wrap a [`ChClient`]. Without it the verb is a structural
    /// no-op.
    pub fn with_client(mut self, client: Arc<ChClient>) -> Self {
        self.client = Some(client);
        self
    }
}

#[async_trait]
impl Tool for WarehouseCleanMinuteTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.clean_minute".to_owned(),
            description: rubix_spi::dto::dataflow::clean_minute::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "lookback_minutes": { "type": "integer", "minimum": 1, "maximum": MAX_LOOKBACK_MINUTES }
                },
                "additionalProperties": true
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseCleanMinuteRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseCleanMinuteRequest: {e}"),
            })?;
        let lookback = req
            .lookback_minutes
            .unwrap_or(DEFAULT_LOOKBACK_MINUTES)
            .clamp(1, MAX_LOOKBACK_MINUTES);

        let mut rows = 0u32;
        let written_at_ms = now_epoch_ms();
        if let Some(client) = &self.client {
            client
                .inner()
                .query(&clean_sql(lookback))
                .execute()
                .await
                .map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;

            // Count the rows the pass landed inside the same
            // window so the diagnostic carries a real number
            // instead of "we ran a query". `count()` over the
            // window is cheap on `ReplacingMergeTree` because the
            // ORDER BY prefix matches the WHERE.
            #[derive(Row, serde::Deserialize)]
            struct CountRow {
                c: u64,
            }
            let count: Vec<CountRow> = client
                .inner()
                .query(&count_sql(lookback))
                .fetch_all::<CountRow>()
                .await
                .map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;
            rows = count.first().map(|r| r.c as u32).unwrap_or(0);

            // Stage 04 — run the deterministic anomaly gate against
            // the freshly-materialised L2 snapshot. Failures here
            // are logged + swallowed so a flaky CH read does not
            // turn a successful cleaner pass into a failed verb;
            // the next tick (60s later) tries again. See
            // `docs/sessions/data-flow/04-anomaly-rules.md`.
            match crate::warehouse::anomaly_gate::run_anomaly_gate(client).await {
                Ok(fired) if fired > 0 => {
                    tracing::info!(
                        target: "rubix.warehouse.anomaly_gate",
                        fired,
                        "anomaly gate dispatched diagnostics",
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::warn!(
                    target: "rubix.warehouse.anomaly_gate",
                    error = %e,
                    "anomaly gate read failed; cleaner pass kept",
                ),
            }
        }

        let summary = if rows == 0 {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.clean.empty")
                    .expect("hard-coded key parses"),
            )
            .with_param("lookback", DiagnosticParam::I64(i64::from(lookback)))
        } else {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.cleaned").expect("hard-coded key parses"),
            )
            .with_param("rows", DiagnosticParam::I64(i64::from(rows)))
            .with_param("lookback", DiagnosticParam::I64(i64::from(lookback)))
            .with_param("at", DiagnosticParam::Timestamp(written_at_ms))
        };

        let response = WarehouseCleanMinuteResponse {
            summary,
            rows,
            lookback_minutes: lookback,
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

/// Build the cleaner `INSERT INTO meter_readings_1m SELECT ...`
/// for a lookback of `lookback_minutes` ending at the most recent
/// fully-elapsed minute (`toStartOfMinute(now()) - INTERVAL 1
/// MINUTE`). Split out so a future integration test can assert
/// the SQL shape without hitting CH.
///
/// Note on `rubix.` prefix: every reference to a base table is
/// fully-qualified so the SQL is safe to send through any client,
/// even one not bound to the rubix database.
pub(crate) fn clean_sql(lookback_minutes: u32) -> String {
    let lookback = lookback_minutes as i64;
    format!(
        "INSERT INTO rubix.meter_readings_1m (tenant_id, meter_id, kind, unit, bucket_start, value, quality) \
         WITH \
           win_end   AS (SELECT toStartOfMinute(now()) - INTERVAL 1 MINUTE  AS t), \
           win_start AS (SELECT toStartOfMinute(now()) - INTERVAL {lookback} MINUTE AS t), \
           med_win_start AS (SELECT toStartOfMinute(now()) - INTERVAL 15 MINUTE AS t), \
           meters AS ( \
             SELECT DISTINCT tenant_id, meter_id, any(kind) AS kind, any(unit) AS unit \
             FROM rubix.meter_readings_raw \
             WHERE epoch_ms >= (toUnixTimestamp((SELECT t FROM med_win_start))) * 1000 \
             GROUP BY tenant_id, meter_id \
           ), \
           buckets AS ( \
             SELECT (SELECT t FROM win_start) + INTERVAL number MINUTE AS bucket_start \
             FROM numbers(0, {lookback}) \
           ), \
           cal AS ( \
             SELECT m.tenant_id, m.meter_id, m.kind, m.unit, b.bucket_start \
             FROM meters m CROSS JOIN buckets b \
             WHERE b.bucket_start <= (SELECT t FROM win_end) \
           ), \
           agg AS ( \
             SELECT \
               tenant_id, meter_id, \
               toStartOfMinute(toDateTime(intDiv(epoch_ms, 1000))) AS bucket_start, \
               avgIf(value, NOT isNaN(value))   AS clean_avg, \
               countIf(isNaN(value))            AS nan_count, \
               count()                          AS row_count \
             FROM rubix.meter_readings_raw \
             WHERE epoch_ms >= (toUnixTimestamp((SELECT t FROM med_win_start))) * 1000 \
             GROUP BY tenant_id, meter_id, bucket_start \
           ), \
           med AS ( \
             SELECT tenant_id, meter_id, \
                    quantileExactIf(0.5)(value, NOT isNaN(value) AND value > 0) AS med_15m \
             FROM rubix.meter_readings_raw \
             WHERE epoch_ms >= (toUnixTimestamp((SELECT t FROM med_win_start))) * 1000 \
             GROUP BY tenant_id, meter_id \
           ), \
           joined AS ( \
             SELECT \
               cal.tenant_id AS tenant_id, cal.meter_id AS meter_id, \
               cal.kind AS kind, cal.unit AS unit, cal.bucket_start AS bucket_start, \
               agg.clean_avg AS clean_avg, agg.nan_count AS nan_count, \
               agg.row_count AS row_count, med.med_15m AS med_15m \
             FROM cal \
             LEFT JOIN agg ON cal.tenant_id = agg.tenant_id \
                          AND cal.meter_id  = agg.meter_id \
                          AND cal.bucket_start = agg.bucket_start \
             LEFT JOIN med ON cal.tenant_id = med.tenant_id \
                          AND cal.meter_id  = med.meter_id \
           ) \
         SELECT \
           tenant_id, meter_id, kind, unit, bucket_start, \
           multiIf( \
             ifNull(row_count, 0) = 0,                                            CAST(NULL AS Nullable(Float64)), \
             ifNull(nan_count, 0) > 0 AND clean_avg IS NULL,                      CAST(NULL AS Nullable(Float64)), \
             med_15m > 0 AND clean_avg > 10 * med_15m,                            CAST(med_15m AS Nullable(Float64)), \
             CAST(clean_avg AS Nullable(Float64)) \
           ) AS value, \
           multiIf( \
             ifNull(row_count, 0) = 0,                                            'missing', \
             ifNull(nan_count, 0) > 0 AND clean_avg IS NULL,                      'nan', \
             med_15m > 0 AND clean_avg > 10 * med_15m,                            'clipped', \
             'ok' \
           ) AS quality \
         FROM joined"
    )
}

/// Count L2 rows inside the same window the INSERT just touched so
/// the diagnostic carries a real number rather than guessing from
/// the request shape. Bound exactly to the cleaner's window so
/// re-running a pass returns a stable count (ReplacingMergeTree
/// dedupes by ORDER BY).
pub(crate) fn count_sql(lookback_minutes: u32) -> String {
    let lookback = lookback_minutes as i64;
    format!(
        "SELECT count() AS c FROM rubix.meter_readings_1m \
         WHERE bucket_start >= toStartOfMinute(now()) - INTERVAL {lookback} MINUTE \
           AND bucket_start <= toStartOfMinute(now()) - INTERVAL 1 MINUTE"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_sql_carries_lookback_and_targets_l2() {
        let sql = clean_sql(5);
        assert!(sql.contains("INSERT INTO rubix.meter_readings_1m"));
        assert!(sql.contains("INTERVAL 5 MINUTE"));
        // Calendar synthesis covers the missing-bucket case.
        assert!(sql.contains("CROSS JOIN buckets"));
        // Spike clip applies the 10x median rule.
        assert!(sql.contains("10 * med_15m"));
        // Quality enum carries every value the stage doc requires.
        for q in ["missing", "nan", "clipped", "ok"] {
            assert!(
                sql.contains(&format!("'{q}'")),
                "quality `{q}` missing from clean_sql output"
            );
        }
    }

    #[tokio::test]
    async fn invoke_without_client_returns_zero_rows() {
        let tool = WarehouseCleanMinuteTool::default();
        let resp: WarehouseCleanMinuteResponse = serde_json::from_value(
            tool.invoke(serde_json::json!({})).await.unwrap(),
        )
        .unwrap();
        assert_eq!(resp.rows, 0);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.clean.empty");
        assert_eq!(resp.lookback_minutes, DEFAULT_LOOKBACK_MINUTES);
    }

    #[tokio::test]
    async fn lookback_is_clamped_into_range() {
        let tool = WarehouseCleanMinuteTool::default();
        let resp: WarehouseCleanMinuteResponse = serde_json::from_value(
            tool.invoke(serde_json::json!({"lookback_minutes": 9999}))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.lookback_minutes, MAX_LOOKBACK_MINUTES);
    }
}
