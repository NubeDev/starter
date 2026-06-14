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
        // WS-14: the extension enablement table (`extensions_enablement`) is
        // owned by the `starter-ext-store-pg` kernel crate, which ships its
        // schema as a bare `sqlx::Migrator`. nexus runs it as its own namespaced
        // source (`ext_store`) so the kernel stays decoupled from nexus's
        // migration runner and version numbers never collide with the `nexus`
        // source. The extension-contributed query-kinds table
        // (`nexus_extension_query_kinds`, migration 1801) is owned by nexus and
        // ships in the `nexus` source above.
        .with_source(MigrationSource {
            name: "ext_store",
            migrator: &starter_ext_store_pg::MIGRATOR,
        })
        // Setup/Automation Builder: the `setup_templates` + `setup_runs` tables
        // (DOCS §5), owned by `starter-store-postgres` behind its `setup`
        // feature and shipped as a namespaced source so its version numbers
        // never collide with the nexus source.
        .with_source(starter_store_postgres::setup::SETUP_MIGRATION_SOURCE)
        // The setup engine persists each run to the durable flow run-store
        // (`PgRunStore` → `runs` + `run_checkpoints`), which is what makes
        // §8a/§8b crash-recovery + resume-from-cursor work. Without this source
        // those tables are absent: every `run_store.start/finish/checkpoint`
        // call fails ("relation \"runs\" does not exist"), the engine
        // logs-and-continues, and a run is mis-projected as completed without
        // its nodes ever executing (the side effects — e.g. the demo's
        // `device_create` persist — silently never run). Pair the `PgRunStore`
        // with its schema here, same namespaced-source pattern as `setup`.
        .with_source(starter_store_postgres::flow::FLOW_MIGRATION_SOURCE)
        .run()
        .await
        .map_err(|e| format!("migrations: {e}"))
}
