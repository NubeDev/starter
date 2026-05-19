//! Run one or more [`super::MigrationSource`]s against a Postgres
//! pool. Each source's progress is recorded in its own
//! `_sqlx_migrations_<source>` table. Mirrors the sqlite crate.

use sqlx::Executor;

use super::source::MigrationSource;
use crate::pool::Pool;

fn validate_source_name(name: &str) -> Result<(), sqlx::migrate::MigrateError> {
    let ok = !name.is_empty()
        && name.len() <= 31
        && name.as_bytes()[0].is_ascii_lowercase()
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
    if ok {
        Ok(())
    } else {
        Err(sqlx::migrate::MigrateError::Source(
            format!("invalid migration source name {name:?}: must match ^[a-z][a-z0-9_]{{0,30}}$")
                .into(),
        ))
    }
}

/// Entry point. Start a fluent migrate chain over `pool`.
pub fn migrate(pool: &Pool) -> Migrate<'_> {
    Migrate {
        pool,
        sources: Vec::new(),
    }
}

/// In-flight migration plan; finalize with [`Self::run`].
pub struct Migrate<'a> {
    pool: &'a Pool,
    sources: Vec<MigrationSource>,
}

impl<'a> Migrate<'a> {
    /// Register one source. Sources run in the order added.
    pub fn with_source(mut self, source: MigrationSource) -> Self {
        self.sources.push(source);
        self
    }

    /// Apply every pending migration from each registered source.
    pub async fn run(self) -> Result<(), sqlx::migrate::MigrateError> {
        let pool = self.pool.sqlx();
        for source in &self.sources {
            validate_source_name(source.name)?;
            apply_source(pool, source).await?;
        }
        Ok(())
    }
}

async fn apply_source(
    pool: &sqlx::PgPool,
    source: &MigrationSource,
) -> Result<(), sqlx::migrate::MigrateError> {
    let table = format!("_sqlx_migrations_{}", source.name);

    let create = format!(
        "CREATE TABLE IF NOT EXISTS {table} (\n\
             version BIGINT PRIMARY KEY,\n\
             description TEXT NOT NULL,\n\
             installed_on TIMESTAMPTZ NOT NULL DEFAULT NOW(),\n\
             checksum BYTEA NOT NULL\n\
         )"
    );
    pool.execute(create.as_str()).await?;

    for migration in source.migrator.iter() {
        let applied: Option<(i64, Vec<u8>)> = sqlx::query_as(&format!(
            "SELECT version, checksum FROM {table} WHERE version = $1"
        ))
        .bind(migration.version)
        .fetch_optional(pool)
        .await?;

        if let Some((_, existing_checksum)) = applied {
            if existing_checksum != migration.checksum.as_ref() {
                return Err(sqlx::migrate::MigrateError::VersionMismatch(
                    migration.version,
                ));
            }
            continue;
        }

        let mut tx = pool.begin().await?;
        tx.execute(migration.sql.as_ref()).await?;
        sqlx::query(&format!(
            "INSERT INTO {table} (version, description, checksum) VALUES ($1, $2, $3)"
        ))
        .bind(migration.version)
        .bind(migration.description.as_ref())
        .bind(migration.checksum.as_ref())
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
    }

    Ok(())
}
