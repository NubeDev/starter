use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tracing::{error, info, warn};

const CLICKHOUSE_URL: &str = "http://localhost:8123";
const CLICKHOUSE_USER: &str = "demo";
const CLICKHOUSE_PASS: &str = "demo";
const CLICKHOUSE_DB: &str = "demo";

// Z-score thresholds
const WARN_THRESHOLD: f64 = 2.0;
const CRIT_THRESHOLD: f64 = 3.5;

// How often to poll
const POLL_INTERVAL_SECS: u64 = 30;

#[derive(Debug, Deserialize)]
struct Anomaly {
    device_id: String,
    location: String,
    metric: String,
    unit: String,
    baseline_avg: f64,
    baseline_std: f64,
    recent_avg: f64,
    z_score: f64,
}

impl Anomaly {
    fn severity(&self) -> &'static str {
        if self.z_score.abs() >= CRIT_THRESHOLD {
            "CRITICAL"
        } else {
            "WARNING"
        }
    }

    fn direction(&self) -> &'static str {
        if self.z_score > 0.0 {
            "HIGH"
        } else {
            "LOW"
        }
    }
}

async fn run_anomaly_query(client: &Client) -> Result<Vec<Anomaly>> {
    // Baseline = everything before the last hour (uses max ts in table as anchor
    // so it works with historical demo data as well as live data).
    // Recent  = last hour of data.
    // Alert when |z-score| > WARN_THRESHOLD.
    // anchor = max(ts) in table so this works with both live and historical demo data.
    // baseline = everything older than 1 hour before anchor (no upper bound on age).
    // recent   = last 1 hour before anchor.
    let sql = format!(
        r#"
WITH
  anchor AS (SELECT max(ts) AS t FROM iot_readings),
  baseline AS (
    SELECT device_id, location, metric, unit,
      avg(value)       AS avg_val,
      stddevPop(value) AS std_val
    FROM iot_readings CROSS JOIN anchor
    WHERE ts < anchor.t - INTERVAL 1 HOUR
    GROUP BY device_id, location, metric, unit
  ),
  recent AS (
    SELECT device_id, metric, avg(value) AS recent_avg
    FROM iot_readings CROSS JOIN anchor
    WHERE ts >= anchor.t - INTERVAL 1 HOUR
    GROUP BY device_id, metric
  )
SELECT
  b.device_id,
  b.location,
  b.metric,
  b.unit,
  round(b.avg_val,    3) AS baseline_avg,
  round(b.std_val,    3) AS baseline_std,
  round(r.recent_avg, 3) AS recent_avg,
  round((r.recent_avg - b.avg_val) / nullIf(b.std_val, 0), 3) AS z_score
FROM baseline b
JOIN recent r ON b.device_id = r.device_id AND b.metric = r.metric
WHERE abs((r.recent_avg - b.avg_val) / nullIf(b.std_val, 0)) > {WARN_THRESHOLD}
ORDER BY abs(z_score) DESC
FORMAT JSONEachRow
"#
    );

    let body = client
        .post(CLICKHOUSE_URL)
        .basic_auth(CLICKHOUSE_USER, Some(CLICKHOUSE_PASS))
        .query(&[("database", CLICKHOUSE_DB)])
        .body(sql)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    // ClickHouse JSONEachRow = one JSON object per line
    let anomalies = body
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(serde_json::from_str::<Anomaly>)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(anomalies)
}

fn print_alert(a: &Anomaly) {
    let sev = a.severity();
    let dir = a.direction();
    let msg = format!(
        "[{sev}] {dev} @ {loc} | {metric} is {dir}: \
         recent={recent:.2}{unit}  baseline={base:.2}±{std:.2}{unit}  z={z:.2}",
        sev = sev,
        dev = a.device_id,
        loc = a.location,
        metric = a.metric,
        dir = dir,
        recent = a.recent_avg,
        unit = a.unit,
        base = a.baseline_avg,
        std = a.baseline_std,
        z = a.z_score,
    );

    if sev == "CRITICAL" {
        error!("{msg}");
    } else {
        warn!("{msg}");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let client = Client::builder().timeout(Duration::from_secs(10)).build()?;

    info!(
        url = CLICKHOUSE_URL,
        db = CLICKHOUSE_DB,
        interval_secs = POLL_INTERVAL_SECS,
        warn_z = WARN_THRESHOLD,
        crit_z = CRIT_THRESHOLD,
        "IoT anomaly detector starting"
    );

    loop {
        info!("Running anomaly scan...");

        match run_anomaly_query(&client).await {
            Ok(anomalies) if anomalies.is_empty() => {
                info!("No anomalies detected.");
            }
            Ok(anomalies) => {
                info!("{} anomaly(s) detected!", anomalies.len());
                for a in &anomalies {
                    print_alert(a);
                }
            }
            Err(e) => {
                error!("Query failed: {e:#}");
            }
        }

        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
    }
}
