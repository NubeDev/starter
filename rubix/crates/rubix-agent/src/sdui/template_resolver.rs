//! Single source of truth for resolving named warehouse-read
//! templates against the Timescale `samples` hypertable.
//!
//! Two consumers call into this module today:
//!
//! 1. [`TimescaleAnalyticsBridge`][super::TimescaleAnalyticsBridge]
//!    — the legacy SDUI chart-source path; tenant arrives in
//!    `params["tenant_id"]` (the SDUI route layer binds it before
//!    dispatch).
//! 2. [`RubixWarehouseReadBackend`][crate::extensions::backends::RubixWarehouseReadBackend]
//!    — the extension-substrate `ctx.warehouse_read().query(…)`
//!    handle; tenant arrives from `ctx.caller().tenant_id` and is
//!    passed in explicitly.
//!
//! Both paths hit the **same** registry-driven dispatch, so adding
//! or removing a template is a one-place change. The registry gate
//! (refusing unknown names) lives in the caller; this module
//! assumes the caller has already verified the template is
//! registered and its `tables` are inside the caller's grant.
//!
//! Per Appendix A of
//! [`docs/proposal/extension-architecture-north-star.md`](../../../../docs/proposal/extension-architecture-north-star.md)
//! and row 2 of `docs/scope/extensions-north-star/README.md`, the
//! resolver receives `tenant_id` as an immutable parameter — there
//! is no path to override it from the caller's request body.

use serde_json::{json, Value as JsonValue};
use starter_store_warehouse::WarehouseClient;

/// Resolve a named template against the warehouse and return the
/// row vector the caller renders / hands back.
///
/// `params` is opaque JSON; only the keys the matched template
/// documents are read (`meter_id` for the bucketed-series
/// templates). `tenant_id` is bound from the *outside* — the
/// extension caller path passes `ctx.caller().tenant_id`; the
/// legacy SDUI path passes `params["tenant_id"]` after lifting it
/// to the top.
///
/// Unknown template names return `Ok(vec![])` so the bridge can
/// log + render no-data without surfacing a hard error to the
/// chart pipeline. (The extension-facing backend treats unknown
/// templates as `Error::Validation` *before* calling this fn, so
/// the empty-vec branch is only hit by the legacy bridge during
/// `cfg.warehouse_url = None` shadow paths.)
pub async fn resolve(
    client: &WarehouseClient,
    template: &str,
    tenant_id: &str,
    params: &JsonValue,
) -> Result<Vec<JsonValue>, String> {
    match template {
        "meter_kwh_last_24h" => latest_for_kind(client, tenant_id, "elec", "kwh").await,
        "meter_litres_last_24h" => latest_for_kind(client, tenant_id, "water", "litres").await,
        "meter_value_30d_15m" => {
            let Some(meter) = param_str(params, "meter_id") else {
                return Err("meter_value_30d_15m: meter_id required".to_owned());
            };
            bucketed_series(client, tenant_id, &meter, "15 minutes", "30 days").await
        }
        "meter_value_24h_1m" => {
            let Some(meter) = param_str(params, "meter_id") else {
                return Err("meter_value_24h_1m: meter_id required".to_owned());
            };
            bucketed_series(client, tenant_id, &meter, "1 minute", "24 hours").await
        }
        _ => Ok(vec![]),
    }
}

/// Names of every template this resolver knows how to execute.
///
/// Used by [`RubixWarehouseReadBackend`][crate::extensions::backends::RubixWarehouseReadBackend]
/// to refuse, with a clear log line, a template that is in the
/// `TemplateRegistry` but has no resolver wired here — the
/// registry-vs-resolver mismatch the legacy bridge's defensive
/// `other` arm already guards against.
pub fn known_template_names() -> &'static [&'static str] {
    &[
        "meter_kwh_last_24h",
        "meter_litres_last_24h",
        "meter_value_30d_15m",
        "meter_value_24h_1m",
    ]
}

fn param_str(params: &JsonValue, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_owned)
}

async fn latest_for_kind(
    client: &WarehouseClient,
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
    .fetch_optional(client.pool())
    .await
    .map_err(|e| format!("samples query: {e}"))?;
    match row {
        Some((v,)) => Ok(vec![json!({ out_field: v })]),
        None => Ok(vec![]),
    }
}

async fn bucketed_series(
    client: &WarehouseClient,
    tenant_id: &str,
    meter_id: &str,
    bucket: &str,
    window: &str,
) -> Result<Vec<JsonValue>, String> {
    // `time_bucket` is a Timescale function. Window is a SQL
    // INTERVAL literal we splice into the query via a bound param.
    // Both `bucket` and `window` are constants supplied by the
    // matched template arm, never user input — no injection vector.
    let rows: Vec<(chrono::DateTime<chrono::Utc>, Option<f64>)> = sqlx::query_as(
        "SELECT time_bucket($1::interval, ts) AS bucket_start, \
                AVG(value_num) AS value_avg \
         FROM samples \
         WHERE tenant_id = $2 \
           AND entity_id = $3 \
           AND ts >= NOW() - $4::interval \
           AND value_num IS NOT NULL \
         GROUP BY bucket_start \
         ORDER BY bucket_start ASC",
    )
    .bind(bucket)
    .bind(tenant_id)
    .bind(meter_id)
    .bind(window)
    .fetch_all(client.pool())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_template_names_match_resolver_arms() {
        // If you add an arm in `resolve()`, you must add the name
        // here so the backend can refuse mis-registered templates
        // with a clear error rather than silently returning `[]`.
        assert_eq!(
            known_template_names(),
            &[
                "meter_kwh_last_24h",
                "meter_litres_last_24h",
                "meter_value_30d_15m",
                "meter_value_24h_1m",
            ]
        );
    }
}
