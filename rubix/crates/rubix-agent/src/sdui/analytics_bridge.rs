//! `TimescaleAnalyticsBridge` — implements
//! [`starter_sdui_routes::AnalyticsBridge`] against the Timescale
//! `samples` hypertable.
//!
//! Resolves the four named templates the bundled `data-flow-site-a`
//! dashboard uses:
//!
//! | template                  | params                    | rows         |
//! |---------------------------|---------------------------|--------------|
//! | `meter_kwh_last_24h`      | `tenant_id`               | `[{kwh}]`    |
//! | `meter_litres_last_24h`   | `tenant_id`               | `[{litres}]` |
//! | `meter_value_30d_15m`     | `tenant_id`, `meter_id`   | `[{bucket_start, value_avg}, …]` |
//! | `meter_value_24h_1m`      | `tenant_id`, `meter_id`   | `[{bucket_start, value_avg}, …]` |
//!
//! KPIs return the most-recent sample for the tenant filtered by
//! `entity_id LIKE '<tenant>.elec.%'` / `.water.%` since the synth
//! tool emits cumulative meter readings (kWh / L) rather than
//! deltas. Charts return `time_bucket()` averages over the requested
//! window.
//!
//! Templates outside this set return an empty row vector — the
//! upstream resolver then renders the chart / KPI as no-data, which
//! is the same outcome as having no bridge at all.

use std::collections::BTreeMap;

use async_trait::async_trait;
use serde_json::{json, Value as JsonValue};
use starter_sdui_routes::AnalyticsBridge;
use starter_store_warehouse::WarehouseClient;
use tracing::warn;

/// Concrete bridge backed by a Timescale `samples` hypertable.
#[derive(Clone)]
pub struct TimescaleAnalyticsBridge {
    client: WarehouseClient,
}

impl TimescaleAnalyticsBridge {
    pub fn new(client: WarehouseClient) -> Self {
        Self { client }
    }

    async fn latest_for_kind(
        &self,
        tenant_id: &str,
        kind_suffix: &str,
        out_field: &str,
    ) -> Result<Vec<JsonValue>, String> {
        let like_pattern = format!("{tenant_id}.{kind_suffix}.%");
        let row: Option<(f64,)> = sqlx::query_as(
            "SELECT value_num \
             FROM samples \
             WHERE tenant_id = $1 \
               AND entity_id LIKE $2 \
               AND value_num IS NOT NULL \
             ORDER BY ts DESC \
             LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&like_pattern)
        .fetch_optional(self.client.pool())
        .await
        .map_err(|e| format!("samples query: {e}"))?;
        match row {
            Some((v,)) => Ok(vec![json!({ out_field: v })]),
            None => Ok(vec![]),
        }
    }

    async fn bucketed_series(
        &self,
        tenant_id: &str,
        meter_id: &str,
        bucket: &str,
        window: &str,
    ) -> Result<Vec<JsonValue>, String> {
        // `time_bucket` is a Timescale function. Window is a SQL
        // INTERVAL literal we splice into the query — both `bucket`
        // and `window` come from the matched template, never user
        // input, so no injection vector. Bind values still go
        // through parameters.
        let sql = format!(
            "SELECT time_bucket($1::interval, ts) AS bucket_start, \
                    AVG(value_num) AS value_avg \
             FROM samples \
             WHERE tenant_id = $2 \
               AND entity_id = $3 \
               AND ts >= NOW() - $4::interval \
               AND value_num IS NOT NULL \
             GROUP BY bucket_start \
             ORDER BY bucket_start ASC"
        );
        let rows: Vec<(chrono::DateTime<chrono::Utc>, Option<f64>)> = sqlx::query_as(&sql)
            .bind(bucket)
            .bind(tenant_id)
            .bind(meter_id)
            .bind(window)
            .fetch_all(self.client.pool())
            .await
            .map_err(|e| format!("samples bucket query: {e}"))?;
        Ok(rows
            .into_iter()
            .filter_map(|(ts, v)| {
                v.map(|n| {
                    json!({
                        "bucket_start": ts.timestamp_millis(),
                        "value_avg": n,
                    })
                })
            })
            .collect())
    }
}

fn param_str(params: &BTreeMap<String, JsonValue>, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(str::to_owned)
}

#[async_trait]
impl AnalyticsBridge for TimescaleAnalyticsBridge {
    async fn invoke(
        &self,
        name: &str,
        params: &BTreeMap<String, JsonValue>,
    ) -> Result<Vec<JsonValue>, String> {
        match name {
            "meter_kwh_last_24h" => {
                let Some(tenant) = param_str(params, "tenant_id") else {
                    return Err("meter_kwh_last_24h: tenant_id required".to_owned());
                };
                self.latest_for_kind(&tenant, "elec", "kwh").await
            }
            "meter_litres_last_24h" => {
                let Some(tenant) = param_str(params, "tenant_id") else {
                    return Err("meter_litres_last_24h: tenant_id required".to_owned());
                };
                self.latest_for_kind(&tenant, "water", "litres").await
            }
            "meter_value_30d_15m" => {
                let Some(tenant) = param_str(params, "tenant_id") else {
                    return Err("meter_value_30d_15m: tenant_id required".to_owned());
                };
                let Some(meter) = param_str(params, "meter_id") else {
                    return Err("meter_value_30d_15m: meter_id required".to_owned());
                };
                self.bucketed_series(&tenant, &meter, "15 minutes", "30 days")
                    .await
            }
            "meter_value_24h_1m" => {
                let Some(tenant) = param_str(params, "tenant_id") else {
                    return Err("meter_value_24h_1m: tenant_id required".to_owned());
                };
                let Some(meter) = param_str(params, "meter_id") else {
                    return Err("meter_value_24h_1m: meter_id required".to_owned());
                };
                self.bucketed_series(&tenant, &meter, "1 minute", "24 hours")
                    .await
            }
            other => {
                warn!(
                    target: "rubix.sdui.analytics_bridge",
                    template = other,
                    "unknown analytics template; returning empty",
                );
                Ok(vec![])
            }
        }
    }
}
