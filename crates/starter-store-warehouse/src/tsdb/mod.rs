//! TimescaleDB backend (Stage 2 of
//! `rubix/docs/proposal/warehouse-engine-swap.md`).
//!
//! This module is intentionally additive — the legacy ClickHouse
//! types in [`crate::client`] and [`crate::store`] remain in
//! place and continue to satisfy the existing tests until Stage 3
//! deletes them. New code targeting the TimescaleDB engine wires
//! through [`WarehouseClient`] and the typed writers below.
//!
//! All time-series tables are PostgreSQL hypertables created with
//! `chunk_time_interval` values pulled from
//! [`crate::chunk_intervals`]. Writes go through
//! [`sqlx::PgPool::copy_in_raw`] for batch throughput; reads use
//! standard sqlx APIs.

pub mod cagg;
pub mod client;
pub mod migrate;
pub mod retention;
pub mod store;
pub mod windowed;

#[cfg(feature = "testing")]
pub mod testing;

pub use client::{WarehouseClient, WarehouseError};
pub use migrate::{run_migrations, TIMESCALE_MIGRATIONS};
