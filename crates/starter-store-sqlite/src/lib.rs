//! # starter-store-sqlite
//!
//! Typed building blocks consumers compose their own repositories from.
//! No `Store` trait — see SCOPE.md R4.
//!
//! - [`pool`] — `Pool` wrapper carrying tracing context.
//! - [`migrate`] — namespaced migration runner.
//! - [`paging`] — cursor encode/decode helpers.
//! - [`testing`] — in-memory pool factory (`feature = "testing"`).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod migrate;
pub mod paging;
pub mod pool;

#[cfg(feature = "testing")]
pub mod testing;

pub use migrate::{migrate, MigrationSource};
pub use pool::Pool;
