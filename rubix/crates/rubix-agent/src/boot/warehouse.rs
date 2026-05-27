//! TimescaleDB warehouse plane wiring.
//!
//! The warehouse and OLTP share a single Postgres instance — Timescale
//! is just an extension. By default this boot reuses the OLTP `PgPool`
//! and calls `WarehouseClient::from_pool`, so the agent runs against
//! exactly one database with one connection pool. A separate
//! `warehouse_url` is only honoured if explicitly set (split-DB
//! deployments), in which case a second pool is opened.

use anyhow::Result;
use sqlx::PgPool;
use starter_store_warehouse::{run_migrations, WarehouseClient};
use tracing::info;

/// Logical "warehouse" database name. Carried so existing call
/// sites that reference it still compile.
pub const RUBIX_CH_DATABASE: &str = "rubix";

/// Outcome of the boot-time migration step.
#[derive(Debug, Clone, Default)]
pub struct WarehouseMigrationReport {
    /// `true` when the step was a no-op.
    pub skipped: bool,
}

/// Legacy no-arg entry point kept for `main.rs`. The historical
/// signature took `(warehouse_url, database_url, clickhouse_pg_url)`.
pub async fn apply_warehouse_migrations(
    _warehouse_url: Option<&str>,
    _database_url: Option<&str>,
    _ignored: Option<&str>,
) -> Result<WarehouseMigrationReport> {
    Ok(WarehouseMigrationReport { skipped: true })
}

/// Wire the warehouse plane. Prefers the shared OLTP pool; opens a
/// dedicated pool only if a distinct `warehouse_url` is configured.
/// Returns `None` only when neither is available (no DB at all).
pub async fn connect_warehouse(
    warehouse_url: Option<&str>,
    oltp_pool: Option<&PgPool>,
) -> Result<Option<WarehouseClient>> {
    let client = match (warehouse_url, oltp_pool) {
        (Some(url), _) => {
            let c = WarehouseClient::connect(url)
                .await
                .map_err(|e| anyhow::anyhow!("connect warehouse {url}: {e}"))?;
            info!(target: "rubix.boot.warehouse", url = %url, "warehouse plane wired (dedicated pool)");
            c
        }
        (None, Some(pool)) => {
            info!(target: "rubix.boot.warehouse", "warehouse plane wired (shared OLTP pool)");
            WarehouseClient::from_pool(pool.clone())
        }
        (None, None) => {
            info!(target: "rubix.boot.warehouse", "no database configured — skipping warehouse wiring");
            return Ok(None);
        }
    };

    run_migrations(&client)
        .await
        .map_err(|e| anyhow::anyhow!("warehouse migrations: {e}"))?;
    Ok(Some(client))
}
