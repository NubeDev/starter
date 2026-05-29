//! Resolves named warehouse-read templates against the Timescale
//! warehouse pool.
//!
//! Two template flavours flow through this module:
//!
//! 1. **Host-builtin templates** (`meter_kwh_last_24h`,
//!    `meter_litres_last_24h`, `meter_value_30d_15m`,
//!    `meter_value_24h_1m`) — the four `samples`-hypertable queries
//!    the legacy SDUI bridge has always answered. These keep their
//!    hand-written arms because the host owns the schema.
//!
//! 2. **Extension-contributed templates** — every
//!    `contributes.warehouse_templates[]` entry the loader folded
//!    into [`TemplateRegistry`]. These have NO per-extension code
//!    here: the resolver delegates to
//!    [`super::contributed_template::execute`], which compiles the
//!    extension-shipped SQL (`spec.sql`) once and runs it through
//!    `sqlx::query` with bound parameters. Adding a new extension
//!    that ships templates is a zero-touch change on the host
//!    (SCOPE R7 — never string-template SQL — and R8 — extensions
//!    are self-contained).
//!
//! Two consumers call into this module:
//!
//! - [`super::TimescaleAnalyticsBridge`] — legacy SDUI chart-source
//!   path; tenant arrives via `params["tenant_id"]`.
//! - [`crate::extensions::backends::RubixWarehouseReadBackend`] —
//!   the extension-substrate `ctx.warehouse_read().query(…)` handle;
//!   tenant arrives from `ctx.caller().tenant_id`.
//!
//! Both paths share the same registry-driven dispatch — adding,
//! removing, or contributing a template is a one-place change.

use serde_json::{json, Value as JsonValue};
use starter_ext_spi::warehouse::TemplateSpec;
use starter_store_warehouse::WarehouseClient;

use super::contributed_template;

/// Resolve a named template against the warehouse.
///
/// `spec` is the registry entry for `template` (the caller looked
/// it up to enforce the grant gate; passing it in saves a second
/// lookup and is the seam through which contributed-template SQL
/// reaches this module). Pass `None` only from paths that
/// definitely cannot have a registered spec — those will only ever
/// match a host-builtin name.
///
/// Unknown template names that aren't builtins and don't carry a
/// spec return `Ok(vec![])` so the legacy SDUI bridge can log + render
/// no-data without surfacing a hard error to the chart pipeline.
pub async fn resolve(
    client: &WarehouseClient,
    template: &str,
    tenant_id: &str,
    params: &JsonValue,
    spec: Option<&TemplateSpec>,
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
        // Anything else is — by definition — a contribution. The
        // host carries no per-extension code (SCOPE R8); we hand
        // the registered spec to the generic compiler/executor.
        _ => match spec {
            Some(spec) => {
                contributed_template::execute(client.pool(), spec, tenant_id, params).await
            }
            None => Ok(vec![]),
        },
    }
}

/// Names of every host-builtin template this resolver knows. Used
/// by [`crate::extensions::backends::RubixWarehouseReadBackend`]
/// and the analytics bridge for log / diagnostic surfaces.
/// Contributed templates are NOT listed here — they are enumerated
/// by walking the live [`starter_ext_host::TemplateRegistry`].
pub fn known_template_names() -> &'static [&'static str] {
    &[
        "meter_kwh_last_24h",
        "meter_litres_last_24h",
        "meter_value_30d_15m",
        "meter_value_24h_1m",
    ]
}

fn param_str(params: &JsonValue, key: &str) -> Option<String> {
    params.get(key).and_then(|v| v.as_str()).map(str::to_owned)
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
    // `time_bucket` is a Timescale function. `bucket` and `window`
    // are constants supplied by the matched arm above — never user
    // input.
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
    fn known_template_names_lists_only_host_builtins() {
        // Contributed templates are NOT listed here — they live in
        // the runtime `TemplateRegistry` and reach the resolver via
        // the `spec` parameter, not a hardcoded arm.
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
