//! # starter-store-postgres
//!
//! Postgres twin of `starter-store-sqlite`. Same shape:
//!
//! - [`pool`] — `Pool` wrapper.
//! - [`migrate`] — namespaced migration runner.
//! - [`paging`] — cursor codec.
//! - [`testing`] — testcontainers-backed pool factory
//!   (`feature = "testing"`).
//!
//! Keeping the public surfaces identical to the SQLite crate's is a
//! load-bearing design choice — see SCOPE.md "Drop Postgres for SQLite"
//! smoke test.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod migrate;
pub mod paging;
pub mod pool;
pub mod windowed;
pub use windowed::PgWindowedFetcher;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "skill-approvals")]
pub mod skills;

#[cfg(feature = "flow")]
pub mod flow;

#[cfg(feature = "dimensions")]
pub mod dimensions;

#[cfg(feature = "dimensions")]
pub use dimensions::DIMENSIONS_MIGRATION_SOURCE;

pub mod scheduled_flows;

pub use scheduled_flows::{SCHEDULED_FLOWS_MIGRATION_SOURCE, SCHEDULED_FLOWS_MIGRATOR};

pub use migrate::{migrate, MigrationSource};
pub use pool::Pool;
