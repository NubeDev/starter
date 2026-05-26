//! `rubix.warehouse.rollup_15m` — tool dispatch.
//!
//! One verb call = one rollup pass: run a single CH `INSERT INTO
//! rubix.meter_readings_15m SELECT ...` over the lookback window
//! ending at the most recent fully-elapsed 15-minute bucket.
//! Idempotent because L3 is `ReplacingMergeTree` ordered by
//! `(tenant_id, meter_id, bucket_start)` — repeated passes refine
//! the same bucket rather than duplicating it.
//!
//! Source rows come from L2 (`rubix.meter_readings_1m`), not L1.
//! L2 is already cleaned (gaps → `missing`, NaN → `nan`, ×10
//! spikes → `clipped`), so the rollup just bins by
//! `toStartOfInterval(bucket_start, INTERVAL 15 MINUTE)` and
//! aggregates: `avg`, `min`, `max` over the non-null values plus
//! a `Map(quality, count)` that preserves the L2 quality mix per
//! 15-minute bucket. Buckets where every L2 row carries
//! `value IS NULL` end up with `value_avg = NULL` so dashboards
//! can distinguish "no data" from "zero".
//!
//! Append-only at the row level (no reversible wiring); operators
//! recover by re-running the rollup over a wider lookback or by
//! dropping the L3 mart and rebuilding.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use rubix_spi::dto::dataflow::rollup_15m::{
    WarehouseRollup15mRequest, WarehouseRollup15mResponse, BUCKET_MINUTES,
    DEFAULT_LOOKBACK_MINUTES,
};
use serde_json::Value;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_store_clickhouse::clickhouse;
use starter_store_clickhouse::clickhouse::Row;
use starter_store_clickhouse::ChClient;

/// Hard upper bound on the lookback knob. A one-shot backfill of
/// 7 days is the widest a developer would reasonably trigger
/// interactively; anything beyond that should drop and rebuild
/// the mart.
pub const MAX_LOOKBACK_MINUTES: u32 = 7 * 24 * 60;

/// Concrete [`Tool`] for `rubix.warehouse.rollup_15m`. Optionally
/// holds a [`ChClient`]; without one the verb still validates input
/// and returns `rows = 0` so unit tests and the no-CH laptop dev
/// path keep working.
#[derive(Default, Clone)]
pub struct WarehouseRollup15mTool {
    client: Option<Arc<ChClient>>,
}

impl std::fmt::Debug for WarehouseRollup15mTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WarehouseRollup15mTool")
            .field("client", &self.client.is_some())
            .finish()
    }
}

impl WarehouseRollup15mTool {
    /// Wrap a [`ChClient`]. Without it the verb is a structural
    /// no-op.
    pub fn with_client(mut self, client: Arc<ChClient>) -> Self {
        self.client = Some(client);
        self
    }
}

#[async_trait]
impl Tool for WarehouseRollup15mTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "rubix.warehouse.rollup_15m".to_owned(),
            description: rubix_spi::dto::dataflow::rollup_15m::DESCRIPTOR
                .purpose
                .to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "lookback_minutes": {
                        "type": "integer",
                        "minimum": BUCKET_MINUTES,
                        "maximum": MAX_LOOKBACK_MINUTES
                    }
                },
                "additionalProperties": true
            }),
        }
    }

    async fn invoke(&self, input: Value) -> Result<Value> {
        let req: WarehouseRollup15mRequest =
            serde_json::from_value(input).map_err(|e| Error::Invalid {
                message: format!("WarehouseRollup15mRequest: {e}"),
            })?;
        let lookback = req
            .lookback_minutes
            .unwrap_or(DEFAULT_LOOKBACK_MINUTES)
            .clamp(BUCKET_MINUTES, MAX_LOOKBACK_MINUTES);

        let mut rows = 0u32;
        let written_at_ms = now_epoch_ms();
        if let Some(client) = &self.client {
            client
                .inner()
                .query(&rollup_sql(lookback))
                .execute()
                .await
                .map_err(|e| Error::Internal {
                    source: Box::new(e),
                })?;

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
        }

        let summary = if rows == 0 {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.rollup.empty")
                    .expect("hard-coded key parses"),
            )
            .with_param("lookback", DiagnosticParam::I64(i64::from(lookback)))
        } else {
            Diagnostic::new(
                MessageKey::parse("rubix.warehouse.rolled_up")
                    .expect("hard-coded key parses"),
            )
            .with_param("rows", DiagnosticParam::I64(i64::from(rows)))
            .with_param("lookback", DiagnosticParam::I64(i64::from(lookback)))
            .with_param("at", DiagnosticParam::Timestamp(written_at_ms))
        };

        let response = WarehouseRollup15mResponse {
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

/// Build the rollup `INSERT INTO meter_readings_15m SELECT ...`
/// for a lookback of `lookback_minutes` ending at the most recent
/// fully-elapsed 15-minute bucket
/// (`toStartOfInterval(now(), 15 minute) - INTERVAL 15 MINUTE`).
/// Split out so a unit test can assert the SQL shape without
/// hitting CH.
///
/// Aggregation rules (locked, stage 05 doc §"L3 mart"):
///
/// - `value_avg` — `avg` over `value` filtered by `value IS NOT NULL`,
///   or `NULL` when every L2 row in the bucket is itself NULL.
/// - `value_min` / `value_max` — `min` / `max` over the same filter.
/// - `quality_mix` — `Map(quality, count)` summing how many L2 rows
///   of each `quality` fell into this 15-minute bucket.
///
/// Note on `rubix.` prefix: every reference to a base table is
/// fully-qualified so the SQL is safe to send through any client,
/// even one not bound to the rubix database.
pub(crate) fn rollup_sql(lookback_minutes: u32) -> String {
    let lookback = lookback_minutes as i64;
    let bucket = BUCKET_MINUTES as i64;
    format!(
        "INSERT INTO rubix.meter_readings_15m \
           (tenant_id, meter_id, kind, unit, bucket_start, \
            value_avg, value_min, value_max, quality_mix) \
         SELECT \
           tenant_id, meter_id, \
           any(kind) AS kind, any(unit) AS unit, \
           toStartOfInterval(bucket_start, INTERVAL {bucket} MINUTE) AS bucket_start, \
           avgIf(value, isNotNull(value))                  AS value_avg, \
           minIf(value, isNotNull(value))                  AS value_min, \
           maxIf(value, isNotNull(value))                  AS value_max, \
           CAST(arrayMap(g -> (g.1, toUInt32(g.2)), \
                         arrayZip(groupArray(quality), \
                                  arrayMap(_ -> 1, groupArray(quality)))) \
                AS Map(LowCardinality(String), UInt32))    AS quality_mix \
         FROM rubix.meter_readings_1m \
         WHERE bucket_start >= toStartOfInterval(now(), INTERVAL {bucket} MINUTE) \
                               - INTERVAL {lookback} MINUTE \
           AND bucket_start <  toStartOfInterval(now(), INTERVAL {bucket} MINUTE) \
         GROUP BY tenant_id, meter_id, bucket_start"
    )
}

/// Count L3 rows inside the window the INSERT just touched so the
/// diagnostic carries a real number rather than guessing from the
/// request shape. Bound exactly to the rollup's window so
/// re-running a pass returns a stable count (ReplacingMergeTree
/// dedupes by ORDER BY).
pub(crate) fn count_sql(lookback_minutes: u32) -> String {
    let lookback = lookback_minutes as i64;
    let bucket = BUCKET_MINUTES as i64;
    format!(
        "SELECT count() AS c FROM rubix.meter_readings_15m \
         WHERE bucket_start >= toStartOfInterval(now(), INTERVAL {bucket} MINUTE) \
                               - INTERVAL {lookback} MINUTE \
           AND bucket_start <  toStartOfInterval(now(), INTERVAL {bucket} MINUTE)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollup_sql_carries_lookback_and_targets_l3() {
        let sql = rollup_sql(30);
        assert!(
            sql.contains("INSERT INTO rubix.meter_readings_15m"),
            "rollup must target the L3 mart; got: {sql}"
        );
        assert!(sql.contains("INTERVAL 30 MINUTE"));
        assert!(
            sql.contains("FROM rubix.meter_readings_1m"),
            "rollup must read L2 (not L1) — L2 is already cleaned",
        );
        assert!(sql.contains("toStartOfInterval"));
        assert!(sql.contains("value_avg"));
        assert!(sql.contains("value_min"));
        assert!(sql.contains("value_max"));
        assert!(sql.contains("quality_mix"));
    }

    #[test]
    fn count_sql_targets_l3_and_carries_lookback() {
        let sql = count_sql(45);
        assert!(sql.contains("FROM rubix.meter_readings_15m"));
        assert!(sql.contains("INTERVAL 45 MINUTE"));
    }

    #[tokio::test]
    async fn invoke_with_no_client_returns_zero_rows_with_empty_summary() {
        let tool = WarehouseRollup15mTool::default();
        let resp: WarehouseRollup15mResponse = serde_json::from_value(
            tool.invoke(serde_json::json!({})).await.unwrap(),
        )
        .unwrap();
        assert_eq!(resp.rows, 0);
        assert_eq!(resp.lookback_minutes, DEFAULT_LOOKBACK_MINUTES);
        assert_eq!(resp.summary.code.as_str(), "rubix.warehouse.rollup.empty");
    }

    #[tokio::test]
    async fn invoke_clamps_lookback_to_max() {
        let tool = WarehouseRollup15mTool::default();
        let resp: WarehouseRollup15mResponse = serde_json::from_value(
            tool.invoke(serde_json::json!({ "lookback_minutes": MAX_LOOKBACK_MINUTES + 999 }))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.lookback_minutes, MAX_LOOKBACK_MINUTES);
    }

    #[tokio::test]
    async fn invoke_clamps_lookback_to_min() {
        let tool = WarehouseRollup15mTool::default();
        let resp: WarehouseRollup15mResponse = serde_json::from_value(
            tool.invoke(serde_json::json!({ "lookback_minutes": 1 }))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(resp.lookback_minutes, BUCKET_MINUTES);
    }
}
