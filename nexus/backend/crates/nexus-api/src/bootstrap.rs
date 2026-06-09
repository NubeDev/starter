//! Run every crate's migrations against the shared metadata database, in order.
//!
//! Three crates own tables in one database — `starter-auth-users` (identity),
//! `starter-authz` (grants), and nexus (datasources/dashboards). Each ships a
//! namespaced migration source, so they share a database without version
//! collisions. Identity runs first (it owns the tenant/user tables the others
//! reference), then authz, then nexus.

use starter_authz::store::AUTHZ_POSTGRES_MIGRATOR;
use starter_store_postgres::{migrate, MigrationSource, Pool};

/// Apply all migrations. Run once at startup before serving.
pub async fn migrate_all(pool: &Pool) -> Result<(), String> {
    migrate(pool)
        .with_source(starter_auth_users::migration::postgres_migration_source())
        .with_source(MigrationSource {
            name: "authz",
            migrator: &AUTHZ_POSTGRES_MIGRATOR,
        })
        .with_source(nexus_store::migrate::source())
        .run()
        .await
        .map_err(|e| format!("migrations: {e}"))
}
