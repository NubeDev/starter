//! W15 — catalog GC.
//!
//! Two-window policy:
//! - `marts`/`cleaners`: prune `quarantined`/`failed` rows older
//!   than [`crate::WarehouseConfig::catalog_gc_age_days_quarantined`]
//!   (default 90 days).
//! - `sandboxes`: prune `promoted` rows older than
//!   [`crate::WarehouseConfig::catalog_gc_age_days_promoted`]
//!   (default 365 days). Promoted sandboxes are kept around as
//!   provenance for the cleaner they were promoted into.
//!
//! Runnable as a daily background task (`spawn_daily`) and via
//! `POST /api/warehouse/gc`.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use starter_store_postgres::pool::Pool;

use crate::WarehouseConfig;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GcReport {
    pub marts: u64,
    pub cleaners: u64,
    pub sandboxes: u64,
}

impl GcReport {
    pub fn total(&self) -> u64 {
        self.marts + self.cleaners + self.sandboxes
    }
}

/// Run GC once. Returns the per-table reap counts.
pub async fn run_once(pool: &Pool, cfg: &WarehouseConfig) -> Result<GcReport, sqlx::Error> {
    let qd = cfg.catalog_gc_age_days_quarantined;
    let pm = cfg.catalog_gc_age_days_promoted;
    let mut report = GcReport::default();

    let qd_interval = format!("{qd} days");
    let m = sqlx::query(
        "DELETE FROM marts \
         WHERE status IN ('quarantined','failed') \
           AND created_at < NOW() - $1::interval",
    )
    .bind(&qd_interval)
    .execute(pool.sqlx())
    .await?;
    report.marts = m.rows_affected();

    let c = sqlx::query(
        "DELETE FROM cleaners \
         WHERE status IN ('quarantined','failed') \
           AND created_at < NOW() - $1::interval",
    )
    .bind(&qd_interval)
    .execute(pool.sqlx())
    .await?;
    report.cleaners = c.rows_affected();

    let pm_interval = format!("{pm} days");
    let s = sqlx::query(
        "DELETE FROM sandboxes \
         WHERE (status = 'failed' AND created_at < NOW() - $1::interval) \
            OR (status = 'promoted' AND created_at < NOW() - $2::interval)",
    )
    .bind(&qd_interval)
    .bind(&pm_interval)
    .execute(pool.sqlx())
    .await?;
    report.sandboxes = s.rows_affected();

    tracing::info!(target: "starter.warehouse.gc",
        marts = report.marts, cleaners = report.cleaners, sandboxes = report.sandboxes,
        "warehouse.gc.completed");
    Ok(report)
}

/// Spawn a daily background task. Cancel by dropping the
/// `JoinHandle`.
pub fn spawn_daily(pool: Pool, cfg: Arc<WarehouseConfig>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if let Err(e) = run_once(&pool, &cfg).await {
                tracing::warn!(target: "starter.warehouse.gc", error = %e, "gc run failed");
            }
            tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
        }
    })
}
