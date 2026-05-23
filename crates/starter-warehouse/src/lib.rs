//! `starter-warehouse` — the warehouse capability crate.
//!
//! See `DOCS/Warehouse/SCOPE.md` for the canonical contract. This
//! crate is default-off. Enable with `--features warehouse` to pull
//! in the Postgres dimensions store, the ClickHouse history store,
//! the flow SPI integration, and the REST surface; enable
//! `--features mcp` additionally for the AI agent / MCP tool
//! surface.
//!
//! The capability owns every W-rule that lives outside the storage
//! crates:
//!
//! - W5  — generated mart DDL (target table + MV), `definition_hash`
//!         catalog enforcement (see [`ddl::mart`] + [`audit`]).
//! - W9  — every warehouse node kind (`tap.write`, `curate.write`,
//!         `bulk.import`, `sandbox.{define,redefine,drop}`,
//!         `cleaner.{define,promote,drop}`,
//!         `mart.{define,read,promote,drop}`) under [`nodes`].
//! - W11 — `dimension_freshness` envelope; HTTP 503 from
//!         `/api/warehouse/status` on `failed_refresh`.
//! - W12 — `ext_manifest_approvals` hash check + re-quarantine
//!         transaction (see [`catalog::ext`]).
//! - W13 — `dictGetOrNull` + `hide_unknown` toggle (see
//!         [`ddl::mart`] and [`nodes::mart_read`]).
//! - W14 — read-time filter validation against the catalog's
//!         promoted columns; structured HTTP 400.
//! - W15 — daily catalog GC background task (see [`gc`]).
//! - W16 — read-after-write bound surfaced via
//!         `/api/warehouse/status.ingest.async_insert_oldest_age_ms`.
//!
//! Hard rules W1, W6, W7, W8 are inherited from the storage crates
//! by construction (split storage; refs-as-FKs; ingest never
//! refuses; `async_insert=1` on every connection).

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "warehouse"), allow(dead_code, unused_imports))]

#[cfg(feature = "warehouse")]
pub mod catalog;
#[cfg(feature = "warehouse")]
pub mod ddl;
#[cfg(feature = "warehouse")]
pub mod dim_freshness;
#[cfg(feature = "warehouse")]
pub mod gc;
#[cfg(feature = "warehouse")]
pub mod audit;
#[cfg(feature = "warehouse")]
pub mod nodes;
#[cfg(feature = "warehouse")]
pub mod rest;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "warehouse")]
pub use catalog::CatalogError;

/// Reverse-DNS kind ids for every warehouse node (W9 enumeration).
/// Tests and external descriptor registrations reference these
/// constants rather than re-typing the strings.
pub mod kinds {
    pub const TAP_WRITE: &str = "starter.warehouse.tap-write";
    pub const CURATE_WRITE: &str = "starter.warehouse.curate-write";
    pub const BULK_IMPORT: &str = "starter.warehouse.bulk-import";
    pub const SANDBOX_DEFINE: &str = "starter.warehouse.sandbox-define";
    pub const SANDBOX_REDEFINE: &str = "starter.warehouse.sandbox-redefine";
    pub const SANDBOX_DROP: &str = "starter.warehouse.sandbox-drop";
    pub const CLEANER_DEFINE: &str = "starter.warehouse.cleaner-define";
    pub const CLEANER_PROMOTE: &str = "starter.warehouse.cleaner-promote";
    pub const CLEANER_DROP: &str = "starter.warehouse.cleaner-drop";
    pub const MART_DEFINE: &str = "starter.warehouse.mart-define";
    pub const MART_READ: &str = "starter.warehouse.mart-read";
    pub const MART_PROMOTE: &str = "starter.warehouse.mart-promote";
    pub const MART_DROP: &str = "starter.warehouse.mart-drop";
}

/// Configurable knobs (`cleaner.sync_backfill_max_rows`,
/// `warehouse.catalog_gc_age_days`, …).
#[derive(Clone, Debug)]
pub struct WarehouseConfig {
    /// RF-6: sync backfill auto-promotes to async beyond this
    /// many source rows. Default 1_000_000.
    pub cleaner_sync_backfill_max_rows: u64,
    /// RF-6: sync backfill auto-promotes to async beyond this many
    /// seconds of wall clock. Default 300 (5 min).
    pub cleaner_sync_backfill_wall_clock_secs: u64,
    /// W15: prune `quarantined`/`failed` rows older than N days.
    /// Default 90.
    pub catalog_gc_age_days_quarantined: i32,
    /// W15: prune `promoted` sandbox rows older than N days.
    /// Default 365.
    pub catalog_gc_age_days_promoted: i32,
    /// W12: live-mart quota.
    pub live_mart_quota: i32,
}

impl Default for WarehouseConfig {
    fn default() -> Self {
        Self {
            cleaner_sync_backfill_max_rows: 1_000_000,
            cleaner_sync_backfill_wall_clock_secs: 300,
            catalog_gc_age_days_quarantined: 90,
            catalog_gc_age_days_promoted: 365,
            live_mart_quota: 50,
        }
    }
}
