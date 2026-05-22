//! # starter-changelog-sqlite
//!
//! SQLite backend for the append-only changelog. Owns the
//! `starter_changes` table and ships the [`SqliteChangeRecorder`],
//! [`SqliteChangeLog`], [`SqliteChangePrune`], and
//! [`SqlitePollingTail`] impls.
//!
//! Wire-up:
//!
//! ```ignore
//! use starter_store_sqlite::migrate;
//! use starter_changelog_sqlite::{migration_source, SqliteChangeRecorder, SqliteChangeLog};
//!
//! migrate(&pool).with_source(migration_source()).run().await?;
//! let recorder = SqliteChangeRecorder::new(pool.clone());
//! let log = SqliteChangeLog::new(pool.clone());
//! ```
//!
//! See `DOCS/backend/undo-redo/SCOPE.md`.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod codec;
mod ids;
mod migration;
mod prune;
mod query;
mod recorder;
mod tail;

pub use migration::{migration_source, CHANGELOG_MIGRATOR};
pub use prune::SqliteChangePrune;
pub use query::SqliteChangeLog;
pub use recorder::{SqliteChangeRecorder, SqliteChangeTx};
pub use tail::SqlitePollingTail;
