//! Run one or more [`super::MigrationSource`]s against a pool.
//!
//! Each source's progress is recorded in
//! `_sqlx_migrations_<source_name>`, so consumer migrations
//! starting at version 1 never collide with starter-owned
//! migrations also starting at version 1.

use super::source::MigrationSource;
use crate::pool::Pool;

/// Apply every pending migration from each given source.
///
/// Sources run in the order supplied. Within a source, sqlx applies
/// versions in ascending order.
///
/// Returns the first error encountered; partially-applied migrations
/// are committed (sqlx's default), so a re-run picks up where the
/// previous attempt left off.
pub async fn migrate(_pool: &Pool, _sources: &[MigrationSource]) -> Result<(), sqlx::Error> {
    // TODO(ap): wire `Migrator::set_table_name` once we settle on
    // the registration API (SCOPE.md open question 2). Until then,
    // the public surface is locked.
    Ok(())
}
