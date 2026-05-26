//! starter-store-warehouse — typed building blocks for the
//! TimescaleDB-backed warehouse history side.
//!
//! As of `rubix/docs/proposal/warehouse-engine-swap.md` stage 3 the
//! ClickHouse engine is gone; the [`tsdb`] submodule is the only
//! surface this crate ships. Its types are re-exported at the
//! crate root for ergonomic consumers.

pub mod chunk_intervals;
pub mod tsdb;

pub use tsdb::{
    cagg, client::WarehouseClient, client::WarehouseError, migrate::run_migrations,
    migrate::TIMESCALE_MIGRATIONS, retention, store,
};

#[cfg(feature = "testing")]
pub mod testing {
    //! Testcontainer factory — Timescale-on-Postgres image.
    pub use crate::tsdb::testing::*;
}
