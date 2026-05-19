//! Run one or more [`super::MigrationSource`]s against a Postgres
//! pool. Each source's progress is recorded in its own
//! `_sqlx_migrations_<source>` table.

use super::source::MigrationSource;
use crate::pool::Pool;

/// Apply every pending migration from each given source.
pub async fn migrate(_pool: &Pool, _sources: &[MigrationSource]) -> Result<(), sqlx::Error> {
    // TODO(ap): see sqlite crate's runner — same TODO, single
    // registration API decision pending.
    Ok(())
}
