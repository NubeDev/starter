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

#[cfg(feature = "testing")]
pub mod testing;

pub use migrate::{migrate, MigrationSource};
pub use pool::Pool;
