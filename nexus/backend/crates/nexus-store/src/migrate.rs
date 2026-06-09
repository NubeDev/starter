//! Run the nexus metadata-store migrations.
//!
//! Migrations are namespaced via `starter_store_postgres`'s multi-source runner,
//! so the `nexus` source has its own version table and never collides with the
//! starter/auth/authz migrations sharing the same database. Ordering: starter's
//! own sources run first (they own the identity tables), then `nexus`.

use sqlx::PgPool;
use starter_spi::Error;
use starter_store_postgres::{migrate, MigrationSource, Pool};

static NEXUS_MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/nexus");

/// The nexus migration source, for callers composing it with starter's sources.
pub fn source() -> MigrationSource {
    MigrationSource {
        name: "nexus",
        migrator: &NEXUS_MIGRATOR,
    }
}

/// Apply the nexus migrations against `pool`. Used by the binary at startup and
/// by tests after standing up a fresh database.
pub async fn run(pool: &PgPool) -> Result<(), Error> {
    let wrapped = Pool::from_sqlx(pool.clone());
    migrate(&wrapped)
        .with_source(source())
        .run()
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}
