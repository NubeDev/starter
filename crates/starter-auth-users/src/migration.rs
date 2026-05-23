//! Migration sources for the namespaced runner.
//!
//! Mirrors the shape used by `starter-changelog-{sqlite,postgres}`
//! so a consumer can chain auth-users migrations into the same
//! `migrate(pool).with_source(...).with_source(...)` plan they use
//! for the rest of their tables. One source per backend, both
//! named `auth_users` — they live in their own
//! `_sqlx_migrations_auth_users` table either way so version
//! numbers never collide with other starter sources.

#[cfg(feature = "sqlite")]
pub use sqlite::{migration_source as sqlite_migration_source, MIGRATOR as SQLITE_MIGRATOR};

#[cfg(feature = "postgres")]
pub use postgres::{migration_source as postgres_migration_source, MIGRATOR as POSTGRES_MIGRATOR};

#[cfg(feature = "sqlite")]
mod sqlite {
    use starter_store_sqlite::MigrationSource;

    /// SQLite migrator for the `starter_auth_users_*` tables.
    pub static MIGRATOR: sqlx::migrate::Migrator =
        sqlx::migrate!("./migrations/starter_auth_users");

    /// Migration source identifier. Lives in its own
    /// `_sqlx_migrations_auth_users` table so version numbers
    /// never collide with other starter sources.
    pub fn migration_source() -> MigrationSource {
        MigrationSource {
            name: "auth_users",
            migrator: &MIGRATOR,
        }
    }
}

#[cfg(feature = "postgres")]
mod postgres {
    use starter_store_postgres::MigrationSource;

    /// Postgres migrator for the `starter_auth_users_*` tables.
    pub static MIGRATOR: sqlx::migrate::Migrator =
        sqlx::migrate!("./migrations_postgres/starter_auth_users");

    /// Migration source identifier. Lives in its own
    /// `_sqlx_migrations_auth_users` table so version numbers
    /// never collide with other starter sources.
    pub fn migration_source() -> MigrationSource {
        MigrationSource {
            name: "auth_users",
            migrator: &MIGRATOR,
        }
    }
}
