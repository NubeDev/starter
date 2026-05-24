//! Bounded retention sweep for `undo_snapshots`.
//!
//! The `Reversible` rubix tools write one row per destructive op
//! and `rubix-store-postgres::UNDO_SNAPSHOTS_MIGRATION_SOURCE`
//! provisions the table. Left unattended that table grows
//! forever; this module owns the lifetime cap.
//!
//! ## Contract
//!
//! For every `(tenant_id, resource_kind, resource_id)` the sweep
//! keeps **the smaller of**
//!
//! - the most recent `cfg.max_rows_per_resource` rows
//!   (default 50, configurable as `[undo] max_rows_per_resource`
//!   in `agent.toml`), or
//! - rows newer than `cfg.max_age_days` days
//!   (default 90, configurable as `[undo] max_age_days`).
//!
//! Both rules are implemented as a single `DELETE`. Superseded
//! rows (the `superseded_at IS NOT NULL` ones the `rubix.undo.last`
//! verb consumed) count toward the per-resource limit so undo
//! history naturally compacts under steady use.
//!
//! ## Schedule
//!
//! [`spawn_undo_sweep`] runs the sweep once at boot (so a fresh
//! deploy that inherits a fat snapshot table immediately
//! reclaims space) and then re-runs every 24h until the returned
//! [`tokio::task::JoinHandle`] is dropped. Drop the handle to
//! stop the loop (e.g. in a graceful-shutdown path).
//!
//! ## DSN handling
//!
//! When `database_url` is unset the boot path skips the migration
//! step and there is no `undo_snapshots` table; this module's
//! [`spawn_undo_sweep`] no-ops in that case so a laptop boot
//! without Postgres stays clean.

use std::time::Duration;

use anyhow::Result;
use starter_store_postgres::pool::{connect, Pool};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::boot::config::UndoConfig;

/// 24h between sweep ticks. Held as a constant — the cadence is
/// a documented contract, not an operator knob; the *limits* are
/// what operators tune.
const SWEEP_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// Run the sweep once and return the number of rows deleted.
///
/// Public so the integration test can drive the sweep
/// deterministically without waiting for the 24h tick. The query
/// is one statement so it runs atomically per resource bucket
/// from the planner's point of view.
pub async fn sweep_once(pool: &Pool, cfg: &UndoConfig) -> Result<u64> {
    // The CTE numbers each resource's rows newest-first; the
    // outer DELETE removes every row whose row-number exceeds
    // the per-resource cap OR whose age exceeds the day cap.
    // `make_interval(days := $2::int)` is the Postgres-idiomatic
    // way to bind a dynamic interval; `$1` is the bigint row cap.
    let sql = r#"
        WITH ranked AS (
            SELECT
                id,
                row_number() OVER (
                    PARTITION BY tenant_id, resource_kind, resource_id
                    ORDER BY created_at DESC
                ) AS rn
            FROM undo_snapshots
        )
        DELETE FROM undo_snapshots u
        USING ranked r
        WHERE u.id = r.id
          AND (
              r.rn > $1
              OR u.created_at < NOW() - make_interval(days => $2)
          )
    "#;

    let result = sqlx::query(sql)
        .bind(cfg.max_rows_per_resource as i64)
        .bind(cfg.max_age_days as i32)
        .execute(pool.sqlx())
        .await?;
    Ok(result.rows_affected())
}

/// Spawn the boot-tick + 24h-tick retention loop. Returns the
/// task handle so callers (production `main.rs`, tests) can
/// abort the loop on shutdown. When `dsn` is `None` this returns
/// `Ok(None)` and logs a warn so the laptop boot stays quiet.
pub async fn spawn_undo_sweep(
    dsn: Option<&str>,
    cfg: UndoConfig,
) -> Result<Option<JoinHandle<()>>> {
    let Some(dsn) = dsn else {
        warn!(
            target: "rubix.boot",
            "Postgres DSN unset — undo_snapshots sweep skipped",
        );
        return Ok(None);
    };

    let pool = connect(dsn)
        .await
        .map_err(|e| anyhow::anyhow!("connect for undo sweep: {e}"))?;

    // Boot-tick sweep runs inline so the agent comes up with a
    // table already inside the retention envelope. Failures are
    // logged but do not abort the boot — the table existing
    // un-swept is preferable to refusing to start.
    match sweep_once(&pool, &cfg).await {
        Ok(deleted) => info!(
            target: "rubix.boot",
            deleted,
            max_rows_per_resource = cfg.max_rows_per_resource,
            max_age_days = cfg.max_age_days,
            "undo_snapshots sweep (boot tick) complete",
        ),
        Err(e) => warn!(
            target: "rubix.boot",
            error = %e,
            "undo_snapshots sweep (boot tick) failed; will retry on next 24h tick",
        ),
    }

    let handle = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SWEEP_INTERVAL);
        // The first `tick` returns immediately by default; we
        // already swept inline above, so consume that initial
        // tick before sleeping for SWEEP_INTERVAL.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match sweep_once(&pool, &cfg).await {
                Ok(deleted) => debug!(
                    target: "rubix.boot.undo_sweep",
                    deleted,
                    "undo_snapshots sweep (24h tick) complete",
                ),
                Err(e) => warn!(
                    target: "rubix.boot.undo_sweep",
                    error = %e,
                    "undo_snapshots sweep (24h tick) failed; will retry next tick",
                ),
            }
        }
    });
    Ok(Some(handle))
}
