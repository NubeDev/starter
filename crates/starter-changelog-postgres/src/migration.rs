//! Migration source for the namespaced runner.

use starter_store_postgres::MigrationSource;

/// Postgres migrator for the `starter_changes` table.
pub static CHANGELOG_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Migration source identifier. Lives in its own
/// `_sqlx_migrations_changelog` table.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "changelog",
        migrator: &CHANGELOG_MIGRATOR,
    }
}
