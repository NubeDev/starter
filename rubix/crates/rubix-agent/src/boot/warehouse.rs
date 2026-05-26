//! Stage 3 of `rubix/docs/proposal/warehouse-engine-swap.md` —
//! the ClickHouse engine has been removed. The previous boot-time
//! migration step (rubix-owned `0002_history`, `0003_meter_readings_raw`,
//! etc., plus the shared `entities_dict` dictionary) was strictly
//! ClickHouse-specific and is gone.
//!
//! A future stage will reintroduce a TimescaleDB-backed migration
//! step that drives `starter_store_warehouse::run_migrations`
//! through the same `boot` entry point.
//!
//! The thin no-op surface below keeps the boot wiring in `main.rs`
//! compiling while the warehouse capability crate is rebuilt.

use anyhow::Result;

/// Logical "warehouse" database name. Unused by the current
/// engine; carried for downstream wiring that still references
/// the constant.
pub const RUBIX_CH_DATABASE: &str = "rubix";

/// Outcome of the (currently no-op) migration step.
#[derive(Debug, Clone, Default)]
pub struct WarehouseMigrationReport {
    /// `true` when the step was a no-op (always today).
    pub skipped: bool,
}

/// No-op stand-in for the historical CH migration step. Kept so
/// `main.rs` can continue to await a single function during boot.
pub async fn apply_warehouse_migrations(
    _warehouse_url: Option<&str>,
    _database_url: Option<&str>,
    _ignored: Option<&str>,
) -> Result<WarehouseMigrationReport> {
    Ok(WarehouseMigrationReport { skipped: true })
}
