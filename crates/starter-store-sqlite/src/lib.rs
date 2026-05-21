//! # starter-store-sqlite
//!
//! Typed building blocks consumers compose their own repositories from.
//! No `Store` trait — see SCOPE.md R4.
//!
//! - [`pool`] — `Pool` wrapper carrying tracing context.
//! - [`migrate`] — namespaced migration runner.
//! - [`paging`] — cursor encode/decode helpers.
//! - [`testing`] — in-memory pool factory (`feature = "testing"`).
//! - [`flow`] — SQLite impls of the starter-flow-spi store seams
//!   (`feature = "flow"`, default-off per D-F3.3 / D-F3.7).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod migrate;
pub mod paging;
pub mod pool;

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(feature = "flow")]
pub mod flow;

#[cfg(feature = "skill-approvals")]
pub mod skills;

pub use migrate::{migrate, MigrationSource};
pub use pool::Pool;
