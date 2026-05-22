//! Migration source for the namespaced runner.

use starter_store_sqlite::MigrationSource;

/// SQLite migrator for the `starter_changes` table.
///
/// Embed into the consumer's `migrate(pool).with_source(...)` chain
/// via [`migration_source`].
pub static CHANGELOG_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// Migration source identifier. Lives in its own `_sqlx_migrations_changelog`
/// table so version numbers never collide with other starter sources.
pub fn migration_source() -> MigrationSource {
    MigrationSource {
        name: "changelog",
        migrator: &CHANGELOG_MIGRATOR,
    }
}
