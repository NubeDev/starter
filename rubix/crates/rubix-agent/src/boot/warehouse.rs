//! TimescaleDB warehouse plane wiring.
//!
//! PR #44 deleted the ClickHouse engine and left the warehouse
//! capability stubbed out. This module rebuilds the minimum needed
//! to make `analytics_template` chart sources resolve against real
//! data:
//!
//! 1. Connect a `sqlx::PgPool` to `cfg.warehouse_url` (Timescale on
//!    `:5434/warehouse` in dev).
//! 2. Run `starter_store_warehouse::run_migrations` so the `samples`
//!    hypertable + indexes exist.
//! 3. Return a `WarehouseClient` handle the boot code threads into
//!    the ingest tool and the SDUI analytics bridge.
//!
//! When `warehouse_url` is `None` the boot is a no-op and the agent
//! still serves dashboards — KPIs and charts just render empty.

use anyhow::Result;
use starter_store_warehouse::{run_migrations, WarehouseClient};
use tracing::info;

/// Logical "warehouse" database name. Carried so existing call
/// sites that reference it still compile.
pub const RUBIX_CH_DATABASE: &str = "rubix";

/// Outcome of the boot-time migration step.
#[derive(Debug, Clone, Default)]
pub struct WarehouseMigrationReport {
    /// `true` when the step was a no-op (no `warehouse_url`).
    pub skipped: bool,
}

/// Legacy no-arg entry point kept for `main.rs`. The historical
/// signature took `(warehouse_url, database_url, clickhouse_pg_url)`;
/// the warehouse-on-Timescale rebuild uses only the warehouse URL.
pub async fn apply_warehouse_migrations(
    _warehouse_url: Option<&str>,
    _database_url: Option<&str>,
    _ignored: Option<&str>,
) -> Result<WarehouseMigrationReport> {
    Ok(WarehouseMigrationReport { skipped: true })
}

/// Connect to the Timescale warehouse DSN and run schema
/// migrations. Returns a `WarehouseClient` callers use for ingest +
/// analytics; returns `None` when no DSN was configured.
pub async fn connect_warehouse(warehouse_url: Option<&str>) -> Result<Option<WarehouseClient>> {
    let Some(url) = warehouse_url else {
        info!(target: "rubix.boot.warehouse", "no warehouse_url configured — skipping Timescale wiring");
        return Ok(None);
    };

    let client = WarehouseClient::connect(url)
        .await
        .map_err(|e| anyhow::anyhow!("connect warehouse {url}: {e}"))?;
    run_migrations(&client)
        .await
        .map_err(|e| anyhow::anyhow!("warehouse migrations: {e}"))?;
    info!(target: "rubix.boot.warehouse", url = %url, "warehouse plane wired");
    Ok(Some(client))
}
