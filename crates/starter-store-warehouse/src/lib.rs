//! starter-store-warehouse — typed building blocks for the
//! warehouse history side. Owns the L1 / L2 history tables, the
//! `entities_dict` bridge, and the typed write paths per ADR-003.
//!
//! See `DOCS/Warehouse/SCOPE.md` for the data-model contract and
//! `DOCS/storage/ADR-003-clickhouse-warehouse.md` for the crate
//! seam (no sqlx; official `clickhouse` Rust crate; one DDL file
//! per migration; `async_insert=1` on every connection).
//!
//! Scope:
//!
//! - [`client`] — thin wrapper around `clickhouse::Client` that
//!   bakes in the W8 async-insert discipline.
//! - [`migrate`] — small in-crate migration runner; the ecosystem
//!   has no `sqlx::migrate` equivalent for CH.
//! - [`store`] — typed write paths per W8. **Raw `INSERT` strings
//!   outside this crate are forbidden** (lint-enforced; see the
//!   `forbid_raw_insert` doctest).
//! - [`dim_freshness`] — the W11 status enum + the
//!   `system.dictionaries` probe.
//! - [`testing`] — `with_clickhouse()` factory, mirrors
//!   `starter_store_postgres::testing::with_database`.

pub mod chunk_intervals;
pub mod client;
pub mod dim_freshness;
pub mod migrate;
pub mod raw;
pub mod store;

/// TimescaleDB backend (Stage 2 of warehouse-engine-swap). Lives
/// alongside the legacy ClickHouse paths; Stage 3 deletes the old
/// surface.
pub mod tsdb;

#[cfg(feature = "testing")]
pub mod testing;

pub use client::{ChClient, ChClientError, ChConfig};
pub use migrate::{MigrationError, MigrationRunner, PgSource, MIGRATION_SOURCE};

/// Re-export of the underlying official `clickhouse` Rust crate so
/// consumers can use the `Row` derive, the typed `Compression` enum
/// and the rest of the low-level surface without taking a direct
/// dependency on `clickhouse` themselves. Pulling everything
/// through `starter-store-warehouse` keeps the workspace's one
/// pin authoritative (ADR-003) and lets consumers like rubix
/// satisfy the "no direct clickhouse dep" rule.
pub use clickhouse;

/// All `.sql` filenames the in-crate runner applies, in order. The
/// runner does the read; this list is exported so callers can audit
/// the planned migration set without touching the filesystem.
pub const MIGRATION_FILES: &[&str] = &[
    "0001_raw_events.sql",
    "0002_samples.sql",
    "0003_events.sql",
    "0004_documents.sql",
    "0005_entities_dict.sql",
];
