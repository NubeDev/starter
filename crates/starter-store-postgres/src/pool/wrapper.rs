//! `Pool` — thin wrapper around `sqlx::PgPool`. Same shape as the
//! sqlite crate's `Pool`.

use sqlx::PgPool;

/// Cloneable handle to the Postgres connection pool.
#[derive(Clone)]
pub struct Pool {
    inner: PgPool,
}

impl Pool {
    /// Wrap a pre-built `sqlx::PgPool`.
    pub fn from_sqlx(inner: PgPool) -> Self {
        Self { inner }
    }

    /// Borrow the underlying `sqlx` pool for consumer queries.
    pub fn sqlx(&self) -> &PgPool {
        &self.inner
    }
}
