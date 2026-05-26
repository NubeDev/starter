//! TimescaleDB connection wrapper.
//!
//! Reuses `sqlx::PgPool` (the same primitive
//! `starter-store-postgres` is built on). Construction is
//! deliberately a thin wrapper rather than a separate `connect`
//! helper so a caller that already holds a `PgPool` (the typical
//! single-database deployment) can hand it straight in.

use sqlx::PgPool;

/// Cloneable handle to a TimescaleDB-backed connection pool.
#[derive(Clone)]
pub struct WarehouseClient {
    pool: PgPool,
}

impl WarehouseClient {
    /// Wrap a pre-built `sqlx::PgPool`. The pool is expected to
    /// be connected to a database that has the `timescaledb`
    /// extension installed.
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Connect to a Postgres URL and return a client. Useful for
    /// tests; production callers typically thread a shared pool
    /// through and use [`Self::from_pool`].
    pub async fn connect(url: &str) -> Result<Self, WarehouseError> {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(16)
            .connect(url)
            .await?;
        Ok(Self::from_pool(pool))
    }

    /// Borrow the underlying pool. Public so DDL paths and
    /// integration tests can issue ad-hoc queries without a
    /// dedicated wrapper for every shape.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Errors surfaced by the TimescaleDB write / read paths.
#[derive(Debug, thiserror::Error)]
pub enum WarehouseError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("serde_json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("other: {0}")]
    Other(String),
}
