//! Install / refresh the `com.nubeio.rubixos` usage continuous
//! aggregate, sqlx-based (no `psql` required).
//!
//! This is the no-`psql` equivalent of
//! `rubix/extensions/com.nubeio.rubixos/scripts/install-caggs.sh`,
//! for environments (like the bring-up host) that have no Postgres
//! client on PATH. It:
//!
//!   1. CREATE MATERIALIZED VIEW … WITH (timescaledb.continuous) — the
//!      `usage_daily_cagg` that backs `usage_bucketed @ '1 day'` and
//!      `usage_site_totals` (≥2-day windows). Idempotent.
//!   2. (re)installs the hourly refresh policy.
//!   3. refreshes the cagg in **bounded windows**, newest-first, so a
//!      huge adopted hypertable doesn't get refreshed in one
//!      multi-hour statement that the shared DB kills mid-flight. Each
//!      window commits on its own.
//!
//! Each step runs as its own simple query on a single connection —
//! continuous-aggregate DDL, `add_continuous_aggregate_policy`, and
//! `CALL refresh_continuous_aggregate` all refuse to run inside a
//! transaction block, so we must NOT wrap them.
//!
//!   RUBIX_PROBE_DSN=postgres://… \
//!     cargo run -p rubix-agent --example install_caggs -- \
//!       [--from 2026-02-01] [--to 2026-05-31] [--step-days 7]
//!
//! With no `--from/--to`, refreshes the trailing `--step-days` window
//! up to "now" only (cheapest; enough to light up the recent ranges
//! the dashboard requests). Give an explicit `--from`/`--to` to
//! backfill more history, walked in `--step-days` chunks newest-first.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

const CAGG: &str = "com_nubeio_rubixos__usage_daily_cagg";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dsn = std::env::var("RUBIX_PROBE_DSN")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let from = arg_value(&args, "--from");
    let to = arg_value(&args, "--to");
    let step_days: i64 = arg_value(&args, "--step-days")
        .as_deref()
        .map(str::parse)
        .transpose()?
        .unwrap_or(7);

    // Long per-statement budget: a bounded refresh window still scans
    // raw chunks. The pool is size 1 so every step reuses one backend.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&dsn)
        .await?;

    // 1. Create the continuous aggregate (no data yet). Idempotent.
    println!("==> CREATE MATERIALIZED VIEW {CAGG} (if not exists)");
    sqlx::query(&format!(
        "CREATE MATERIALIZED VIEW IF NOT EXISTS \"{CAGG}\" \
         WITH (timescaledb.continuous) AS \
         SELECT tenant_id, point_uuid, host_uuid, \
                time_bucket('1 day'::interval, \"timestamp\") AS bucket, \
                AVG(value)::float8 AS avg_value, \
                MIN(value)::float8 AS min_value, \
                MAX(value)::float8 AS max_value, \
                COUNT(*)           AS sample_count \
         FROM com_nubeio_rubixos__histories \
         GROUP BY tenant_id, point_uuid, host_uuid, bucket \
         WITH NO DATA"
    ))
    .execute(&pool)
    .await?;

    // 2. (Re)install the hourly refresh policy, idempotently.
    println!("==> add_continuous_aggregate_policy (hourly)");
    sqlx::query(&format!(
        "DO $$ DECLARE j integer; BEGIN \
           FOR j IN SELECT job_id FROM timescaledb_information.jobs \
             WHERE proc_name='policy_refresh_continuous_aggregate' \
               AND hypertable_name='{CAGG}' \
           LOOP PERFORM remove_continuous_aggregate_policy('{CAGG}'); END LOOP; \
         END $$"
    ))
    .execute(&pool)
    .await?;
    sqlx::query(&format!(
        "SELECT add_continuous_aggregate_policy('{CAGG}', \
           start_offset => INTERVAL '60 days', \
           end_offset => INTERVAL '1 hour', \
           schedule_interval => INTERVAL '1 hour')"
    ))
    .execute(&pool)
    .await?;

    // 3. Bounded refresh, newest-first. Resolve the window: explicit
    //    --from/--to, else the trailing step ending at the data's max
    //    timestamp.
    let (lo, hi): (String, String) = match (from, to) {
        (Some(f), Some(t)) => (f, t),
        _ => {
            let max_ts: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
                "SELECT max(\"timestamp\") FROM com_nubeio_rubixos__histories",
            )
            .fetch_one(&pool)
            .await?;
            let hi = max_ts.unwrap_or_else(chrono::Utc::now);
            let lo = hi - chrono::Duration::days(step_days);
            (
                lo.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                hi.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            )
        }
    };

    println!("==> refresh {CAGG} over [{lo}, {hi}) in {step_days}-day windows (newest first)");
    let lo_ts = chrono::DateTime::parse_from_rfc3339(&lo)?.with_timezone(&chrono::Utc);
    let hi_ts = chrono::DateTime::parse_from_rfc3339(&hi)?.with_timezone(&chrono::Utc);
    let mut win_hi = hi_ts;
    while win_hi > lo_ts {
        let win_lo = std::cmp::max(lo_ts, win_hi - chrono::Duration::days(step_days));
        println!("    refresh [{win_lo}, {win_hi})");
        sqlx::query(&format!(
            "CALL refresh_continuous_aggregate('{CAGG}', $1::timestamptz, $2::timestamptz)"
        ))
        .bind(win_lo)
        .bind(win_hi)
        .execute(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("refresh [{win_lo}, {win_hi}) failed: {e}"))?;
        win_hi = win_lo;
    }

    let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM \"{CAGG}\""))
        .fetch_one(&pool)
        .await?;
    println!("==> {CAGG} ready: {rows} rows");
    pool.close().await;
    Ok(())
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}
