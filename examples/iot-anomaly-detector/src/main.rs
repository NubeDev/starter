//! `iot-anomaly-detector` — worked-example port (Warehouse SCOPE
//! post-review walkthrough).
//!
//! The binary is a thin flow driver:
//!
//! 1. Connect Postgres (dimensions catalog) + ClickHouse (history).
//! 2. Apply the `dimensions` migrations and the CH migrations.
//! 3. Bootstrap an inline `starter-ext-iot` extension shape —
//!    register a manifest hash in `ext_manifest_approvals` (W12),
//!    upsert IoT entities, then `mart.define` `mart_iot_1m` and
//!    `mart_iot_1h` against `samples`.
//! 4. Subscribe to MQTT (`rumqttc`) and, for every message, call
//!    `WarehouseRuntime::tap_write` (W7 — never refuses) plus
//!    `curate_write_sample` (writes into `samples` after the PG
//!    entity lookup; W6 ref-as-FK).
//! 5. Every 30s, run two `mart.read` calls (1m + 1h) and compute
//!    the z-score in-process; emit a `Verdict` per anomaly.
//!
//! Hard rule: **no direct ClickHouse SQL anywhere in this binary**.
//! Every CH read or write flows through `starter-warehouse` →
//! `starter-store-warehouse`. Verify with:
//!
//! ```text
//! cargo tree -p iot-anomaly-detector | grep -i clickhouse
//! # clickhouse v0.13.x          ← reachable via starter-store-warehouse
//! ```
//!
//! No `clickhouse = "…"` line on this crate's `Cargo.toml`.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use rumqttc::{AsyncClient, Event, EventLoop, MqttOptions, Packet, QoS};
use serde::Deserialize;
use serde_json::json;
use starter_store_postgres::dimensions as dim;
use starter_store_warehouse::{ChConfig, PgSource};
use starter_tags::TagQuery;
use starter_warehouse::catalog::mart_spec::{AggregationSpec, MartSpec};
use starter_warehouse::nodes::runtime::WarehouseRuntime;
use starter_warehouse::WarehouseConfig;
use std::str::FromStr;
use tracing::{error, info, warn};

/// In-process anomaly thresholds. The full worked example would
/// emit these as `compute.zscore` flow-node parameters; for the
/// purpose of the smoke we keep them local.
const WARN_Z: f64 = 2.0;
const CRIT_Z: f64 = 3.5;

/// Manifest hash for the inline `starter-ext-iot` extension. The
/// hash is recorded in `ext_manifest_approvals` (W12) so the
/// extension's mart definitions are accepted; a future stage would
/// bump this hash whenever the extension's `manifest.yaml`
/// changes, which (per W12) re-quarantines any live ext-authored
/// mart in the same txn.
const EXT_ID: &str = "starter-ext-iot";
const EXT_MANIFEST_HASH: &str = "iot-ext-v1";

#[derive(Debug, Deserialize)]
struct MqttSample {
    device_id: String,
    location: Option<String>,
    metric: String,
    unit: Option<String>,
    value: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(filter).unwrap_or_else(|_| "info".into()),
        )
        .init();

    let database_url = std::env::var("DATABASE_URL").with_context(|| {
        "DATABASE_URL is required (e.g. postgres://postgres:postgres@localhost:5432/iot)"
    })?;
    let ch_url = std::env::var("CLICKHOUSE_URL").unwrap_or_else(|_| "http://127.0.0.1:8123".into());
    let ch_db = std::env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "default".into());
    let ch_user = std::env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".into());
    let ch_pass = std::env::var("CLICKHOUSE_PASSWORD").unwrap_or_default();
    let mqtt_host = std::env::var("MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let mqtt_port: u16 = std::env::var("MQTT_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1883);
    let mqtt_topic = std::env::var("MQTT_TOPIC").unwrap_or_else(|_| "iot/+/+".into());

    // --- 1. Connect PG + apply dimensions migrations ---------------
    let pool = starter_store_postgres::pool::connect(&database_url)
        .await
        .with_context(|| format!("connect to {database_url}"))?;
    starter_store_postgres::migrate(&pool)
        .with_source(dim::DIMENSIONS_MIGRATION_SOURCE)
        .run()
        .await
        .context("apply dimensions migrations")?;
    info!("postgres: dimensions migrations applied");

    // --- 2. Connect CH + apply CH migrations -----------------------
    let pg_src = pg_source_from_url(&database_url)?;
    let ch_cfg = ChConfig {
        url: ch_url.clone(),
        database: ch_db,
        user: ch_user,
        password: ch_pass,
        async_insert: true,
    };
    let ch_client = starter_store_warehouse::ChClient::connect(ch_cfg.clone());
    starter_store_warehouse::MigrationRunner::new(&ch_client)
        .with_pg_source(pg_src)
        .run()
        .await
        .context("apply clickhouse migrations")?;
    info!("clickhouse: migrations applied");

    // --- 3. WarehouseRuntime + ext approval + mart_define ----------
    let rt = Arc::new(WarehouseRuntime::new(
        pool.clone(),
        ch_cfg,
        WarehouseConfig::default(),
    ));

    // W12: record the inline extension's manifest-hash approval so
    // its mart definitions are not quarantined on creation.
    let mut conn = pool.sqlx().acquire().await.context("acquire pg conn")?;
    starter_warehouse::catalog::ext::record_approval(
        &mut conn,
        EXT_ID,
        EXT_MANIFEST_HASH,
        "install:iot-anomaly-detector",
    )
    .await
    .context("record ext manifest approval")?;
    drop(conn);
    info!(
        ext = EXT_ID,
        hash = EXT_MANIFEST_HASH,
        "warehouse: ext approval recorded"
    );

    // Define mart_iot_1m and mart_iot_1h. Both are
    // AggregatingMergeTree-backed and live behind W14 — the read
    // path validates filter keys against `group_by`.
    for (name, bucket) in [("mart_iot_1m", 60i64), ("mart_iot_1h", 3600)] {
        let spec = MartSpec {
            name: name.into(),
            description: Some("IoT samples rolled up to ".to_string() + name),
            source_table: "samples".into(),
            filter: json!({}),
            time_bucket_secs: bucket,
            group_by: vec!["device_id".into(), "location".into(), "metric".into()],
            aggregations: vec![
                AggregationSpec {
                    func: "avg".into(),
                    col: "value_num".into(),
                    alias: "value_avg".into(),
                },
                AggregationSpec {
                    func: "stddevPop".into(),
                    col: "value_num".into(),
                    alias: "value_std".into(),
                },
                AggregationSpec {
                    func: "count".into(),
                    col: "value_num".into(),
                    alias: "n".into(),
                },
            ],
            created_by: format!("ext:{EXT_ID}"),
            ext_manifest_hash: Some(EXT_MANIFEST_HASH.into()),
        };
        match rt.mart_define(spec).await {
            Ok(r) => {
                info!(name = %r.name, status = %r.status, idempotent = r.idempotent_noop, "mart defined")
            }
            Err(e) => warn!(name, error = %e, "mart_define failed (continuing)"),
        }
    }

    // --- 4. MQTT subscribe → tap_write → curate_write -------------
    let mut mqtt_opts = MqttOptions::new("iot-anomaly-detector", &mqtt_host, mqtt_port);
    mqtt_opts.set_keep_alive(Duration::from_secs(30));
    let (mqtt_client, mqtt_eventloop) = AsyncClient::new(mqtt_opts, 64);
    mqtt_client
        .subscribe(&mqtt_topic, QoS::AtMostOnce)
        .await
        .context("mqtt subscribe")?;
    info!(
        host = mqtt_host,
        port = mqtt_port,
        topic = mqtt_topic,
        "mqtt subscribed"
    );

    // Spawn MQTT ingest loop.
    let rt_ingest = rt.clone();
    tokio::spawn(ingest_loop(mqtt_eventloop, rt_ingest));

    // --- 5. Periodic mart.read + verdict emission ------------------
    let mut ticker = tokio::time::interval(Duration::from_secs(30));
    ticker.tick().await; // skip immediate fire
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if let Err(e) = scan_for_anomalies(&rt).await {
                    error!(error = %e, "anomaly scan failed");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutdown requested");
                break Ok(());
            }
        }
    }
}

async fn ingest_loop(mut el: EventLoop, rt: Arc<WarehouseRuntime>) {
    loop {
        match el.poll().await {
            Ok(Event::Incoming(Packet::Publish(p))) => {
                // W7 — tap.write never refuses payload structure;
                // malformed JSON still lands in raw_events tagged
                // with parse_error.
                let payload = String::from_utf8_lossy(&p.payload).to_string();
                let topic = p.topic.clone();
                let parsed: Option<MqttSample> = serde_json::from_str(&payload).ok();

                let mut raw_tags: Vec<(String, String)> = vec![
                    ("source".to_string(), "mqtt".to_string()),
                    ("topic".to_string(), topic.clone()),
                ];
                if parsed.is_none() {
                    raw_tags.push(("parse_error".to_string(), "1".to_string()));
                }
                if let Err(e) = rt.tap_write("mqtt", payload, raw_tags).await {
                    warn!(error = %e, "tap_write failed");
                    continue;
                }
                if let Some(s) = parsed {
                    // Upsert the entity dimension row + curate into
                    // samples. W6: refs are FKs — entity_id must
                    // exist before `curate.write` accepts the row.
                    let ent_id = format!("device:{}", s.device_id);
                    let tags_json = json!({
                        "device_id": s.device_id,
                        "location": s.location.clone().unwrap_or_default(),
                        "metric": s.metric,
                        "unit": s.unit.clone().unwrap_or_default(),
                    });
                    if let Err(e) = dim::entities::upsert(
                        &rt.pg,
                        &ent_id,
                        "device",
                        Some(&s.device_id),
                        &tags_json,
                    )
                    .await
                    {
                        warn!(error = %e, "entities.upsert failed");
                        continue;
                    }
                    let mut tags: Vec<(String, String)> = vec![
                        ("device_id".into(), s.device_id.clone()),
                        ("metric".into(), s.metric.clone()),
                    ];
                    if let Some(loc) = s.location {
                        tags.push(("location".into(), loc));
                    }
                    if let Err(e) = rt
                        .curate_write_sample(&ent_id, Utc::now(), Some(s.value), tags)
                        .await
                    {
                        warn!(error = %e, "curate_write_sample failed");
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                warn!(error = %e, "mqtt eventloop error; backing off");
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Two `mart.read` calls + an in-process z-score. W14 enforces the
/// filter against `group_by`; W11's `dimension_freshness` rides on
/// the envelope so a downstream consumer can short-circuit reads
/// when the entities_dict has not refreshed.
async fn scan_for_anomalies(rt: &WarehouseRuntime) -> Result<()> {
    // No filter restriction — empty TagQuery (matches everything).
    let q = TagQuery::from_str("").unwrap_or_else(|_| TagQuery::And(vec![]));

    let now = Utc::now();
    let baseline = rt
        .mart_read(
            "mart_iot_1h",
            q.clone(),
            now - chrono::Duration::days(1),
            now - chrono::Duration::hours(1),
            false,
            20_000,
        )
        .await
        .context("mart_iot_1h read")?;

    let recent = rt
        .mart_read(
            "mart_iot_1m",
            q,
            now - chrono::Duration::hours(1),
            now,
            false,
            20_000,
        )
        .await
        .context("mart_iot_1m read")?;

    info!(
        freshness = ?baseline.dimension_freshness,
        baseline_rows = baseline.rows.len(),
        recent_rows = recent.rows.len(),
        "anomaly scan: dimension_freshness envelope observed",
    );

    // Inline `compute.zscore`. The real flow node would index by
    // `(device_id, metric)` and emit Verdicts onto the engine bus;
    // here we just print them.
    let n_anomalies = compute_verdicts(&baseline.rows, &recent.rows);
    if n_anomalies > 0 {
        info!(n_anomalies, "anomalies emitted");
    } else {
        info!("no anomalies");
    }
    Ok(())
}

/// Pure function for the `compute.zscore` flow node. Returns the
/// number of (warn|crit) verdicts emitted. Operates over the
/// JSON rows returned by `mart.read` — the format mirrors
/// `mart_iot_1{m,h}`'s `(device_id, location, metric, value_avg,
/// value_std, n)` shape.
fn compute_verdicts(baseline: &[serde_json::Value], recent: &[serde_json::Value]) -> usize {
    use std::collections::HashMap;
    let mut by_key: HashMap<(String, String), (f64, f64)> = HashMap::new();
    for row in baseline {
        let device = row.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
        let metric = row.get("metric").and_then(|v| v.as_str()).unwrap_or("");
        let avg = row.get("value_avg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let std = row.get("value_std").and_then(|v| v.as_f64()).unwrap_or(0.0);
        by_key.insert((device.into(), metric.into()), (avg, std));
    }
    let mut n = 0usize;
    for row in recent {
        let device = row.get("device_id").and_then(|v| v.as_str()).unwrap_or("");
        let metric = row.get("metric").and_then(|v| v.as_str()).unwrap_or("");
        let recent_avg = row.get("value_avg").and_then(|v| v.as_f64()).unwrap_or(0.0);
        if let Some((avg, std)) = by_key.get(&(device.into(), metric.into())) {
            if *std <= f64::EPSILON {
                continue;
            }
            let z = (recent_avg - avg) / std;
            if z.abs() >= CRIT_Z {
                warn!(device, metric, z, "Verdict CRITICAL");
                n += 1;
            } else if z.abs() >= WARN_Z {
                warn!(device, metric, z, "Verdict WARN");
                n += 1;
            }
        }
    }
    n
}

/// Naive parse of `postgres://user:pass@host:port/db` into the
/// `PgSource` the CH dictionary needs. The CH dictionary connects
/// to PG over its own network so the `WAREHOUSE_PG_HOST`
/// environment override exists for docker-compose-style setups
/// where the CH container reaches PG at a different name.
fn pg_source_from_url(url: &str) -> Result<PgSource> {
    let host = std::env::var("WAREHOUSE_PG_HOST").ok();
    let port_env = std::env::var("WAREHOUSE_PG_PORT").ok();
    let rest = url
        .strip_prefix("postgres://")
        .or_else(|| url.strip_prefix("postgresql://"))
        .context("DATABASE_URL must start with postgres://")?;
    let (creds, hostpart) = rest.split_once('@').unwrap_or(("postgres:postgres", rest));
    let (user, password) = creds.split_once(':').unwrap_or((creds, ""));
    let (host_port, db) = hostpart.split_once('/').unwrap_or((hostpart, "postgres"));
    let (h, p) = host_port.split_once(':').unwrap_or((host_port, "5432"));
    let host = host.unwrap_or_else(|| h.to_string());
    let port: u16 = port_env
        .as_deref()
        .unwrap_or(p)
        .parse()
        .context("WAREHOUSE_PG_PORT / DATABASE_URL port")?;
    Ok(PgSource {
        host,
        port,
        user: user.into(),
        password: password.into(),
        db: db.split('?').next().unwrap_or(db).into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_score_emits_verdicts_above_threshold() {
        // baseline avg=10, std=1, recent avg=15 → z=5 → CRIT.
        let baseline = vec![json!({
            "device_id": "dev-a",
            "metric": "temp",
            "value_avg": 10.0,
            "value_std": 1.0,
        })];
        let recent = vec![json!({
            "device_id": "dev-a",
            "metric": "temp",
            "value_avg": 15.0,
        })];
        assert_eq!(compute_verdicts(&baseline, &recent), 1);
    }

    #[test]
    fn z_score_silent_below_threshold() {
        let baseline = vec![json!({
            "device_id": "dev-a",
            "metric": "temp",
            "value_avg": 10.0,
            "value_std": 5.0,
        })];
        let recent = vec![json!({
            "device_id": "dev-a",
            "metric": "temp",
            "value_avg": 11.0,
        })];
        assert_eq!(compute_verdicts(&baseline, &recent), 0);
    }

    #[test]
    fn pg_source_url_parse() {
        let src = pg_source_from_url("postgres://u:p@h:5433/db").unwrap();
        assert_eq!(src.user, "u");
        assert_eq!(src.password, "p");
        assert_eq!(src.host, "h");
        assert_eq!(src.port, 5433);
        assert_eq!(src.db, "db");
    }
}
