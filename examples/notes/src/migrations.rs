//! Two namespaced migration sources: starter-auth-token's tables
//! and our own `notes` table. Both apply through the single
//! starter-store-sqlite runner — each lands in its own
//! `_sqlx_migrations_<name>` table so version counters never collide.

use starter_store_sqlite::MigrationSource;

static AUTH_TOKEN: sqlx::migrate::Migrator =
    sqlx::migrate!("../../crates/starter-auth-token/migrations/starter_auth_token");

static NOTES: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/notes");

pub fn sources() -> [MigrationSource; 2] {
    [
        MigrationSource { name: "starter_auth_token", migrator: &AUTH_TOKEN },
        MigrationSource { name: "notes", migrator: &NOTES },
    ]
}
