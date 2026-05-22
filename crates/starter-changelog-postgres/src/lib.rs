//! # starter-changelog-postgres
//!
//! Postgres backend for the append-only changelog. Owns the
//! `starter_changes` table.
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
mod tail_listen;

pub use migration::{migration_source, CHANGELOG_MIGRATOR};
pub use prune::PgChangePrune;
pub use query::PgChangeLog;
pub use recorder::{PgChangeRecorder, PgChangeTx};
pub use tail::PgPollingTail;
pub use tail_listen::PgListenTail;
