//! Create + refresh the `com_nubeio_rubixos__histories_1m` continuous
//! aggregate, sqlx-based (no `psql`).
//!
//! This is the no-`psql` equivalent of the CAGG section of
//! `rubix/extensions/com.nubeio.rubixos/scripts/post-load.sql`, for
//! hosts with no Postgres client on PATH. It is the fast path behind
//! the `usage_*` and `history_bucketed_1m` warehouse templates.
//!
//! Why a dedicated runner: on a memory-constrained shared TimescaleDB
//! (the adopted prod box has ~12 MB `work_mem`, ~500 MB
//! `shared_buffers`), `refresh_continuous_aggregate` over a wide
//! window is OOM-killed server-side (the client sees
//! `expected to read 5 bytes, got 0 bytes at EOF`). Refreshing **one
//! day at a time** keeps each refresh's hash state small enough to
//! survive — empirically reliable where a single 7-day call dies.
//!
//!   RUBIX_PROBE_DSN=postgres://… \
//!     cargo run -p rubix-agent --example refresh_histories_1m -- \
//!       --from 2026-05-22 --to 2026-05-30 [--policy]
//!
//! Walks [from, to) in 1-day windows, newest-first. `--policy` also
//! (re)installs the hourly refresh policy. With no `--from/--to`,
//! refreshes the trailing `--days N` (default 7) ending at the data's
//! max timestamp.

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

const CAGG: &str = "com_nubeio_rubixos__histories_1m";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dsn = std::env::var("RUBIX_PROBE_DSN")?;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let from = arg(&args, "--from");
    let to = arg(&args, "--to");
    let days: i64 = arg(&args, "--days").as_deref().map(str::parse).transpose()?.unwrap_or(7);
    let with_policy = args.iter().any(|a| a == "--policy");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&dsn)
        .await?;

    println!("==> CREATE MATERIALIZED VIEW {CAGG} (if not exists)");
    sqlx::query(&format!(
        "CREATE MATERIALIZED VIEW IF NOT EXISTS public.{CAGG} \
         WITH (timescaledb.continuous) AS \
         SELECT tenant_id, point_uuid, host_uuid, \
                time_bucket('1 minute'::interval, \"timestamp\") AS bucket, \
                avg(value)::float8 AS avg_value, \
                min(value)::float8 AS min_value, \
                max(value)::float8 AS max_value, \
                count(*)           AS sample_count \
         FROM public.com_nubeio_rubixos__histories \
         GROUP BY tenant_id, point_uuid, host_uuid, bucket \
         WITH NO DATA"
    ))
    .execute(&pool)
    .await?;

    if with_policy {
        println!("==> add_continuous_aggregate_policy (hourly)");
        sqlx::query(&format!(
            "SELECT add_continuous_aggregate_policy('public.{CAGG}', \
               start_offset => INTERVAL '7 days', \
               end_offset => INTERVAL '1 minute', \
               schedule_interval => INTERVAL '1 minute', \
               if_not_exists => true)"
        ))
        .execute(&pool)
        .await?;
    }

    // Resolve [lo, hi).
    let (lo, hi) = match (from, to) {
        (Some(f), Some(t)) => (parse_day(&f)?, parse_day(&t)?),
        _ => {
            let max_ts: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
                "SELECT max(\"timestamp\") FROM com_nubeio_rubixos__histories",
            )
            .fetch_one(&pool)
            .await?;
            let hi = max_ts.unwrap_or_else(chrono::Utc::now);
            (hi - chrono::Duration::days(days), hi)
        }
    };

    println!("==> refresh {CAGG} over [{lo}, {hi}) one day at a time (newest first)");
    let mut win_hi = hi;
    while win_hi > lo {
        let win_lo = std::cmp::max(lo, win_hi - chrono::Duration::days(1));
        print!("    [{win_lo} → {win_hi}) ... ");
        sqlx::query(&format!(
            "CALL refresh_continuous_aggregate('public.{CAGG}', $1::timestamptz, $2::timestamptz)"
        ))
        .bind(win_lo)
        .bind(win_hi)
        .execute(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("refresh [{win_lo}, {win_hi}) failed (try a smaller step / off-peak): {e}"))?;
        println!("ok");
        win_hi = win_lo;
    }

    let rows: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {CAGG}"))
        .fetch_one(&pool)
        .await?;
    println!("==> {CAGG} ready: {rows} rows");
    pool.close().await;
    Ok(())
}

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

fn parse_day(s: &str) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    // Accept a bare date (YYYY-MM-DD) or full RFC3339.
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).unwrap().and_utc());
    }
    Ok(chrono::DateTime::parse_from_rfc3339(s)?.with_timezone(&chrono::Utc))
}
