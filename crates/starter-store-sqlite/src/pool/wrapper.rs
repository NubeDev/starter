//! `Pool` — a thin wrapper around `sqlx::SqlitePool`.
//!
//! The wrapper exists so we have a place to attach observability
//! plumbing later (per-query tracing spans, slow-query logs) without
//! touching every call site.

use sqlx::SqlitePool;

/// Cloneable handle to the SQLite connection pool.
///
/// `Pool` is `Clone` and cheap to pass around — the underlying
/// `sqlx::SqlitePool` is already arc'd internally.
#[derive(Clone)]
pub struct Pool {
    inner: SqlitePool,
}

impl Pool {
    /// Wrap a pre-built `sqlx::SqlitePool`.
    pub fn from_sqlx(inner: SqlitePool) -> Self {
        Self { inner }
    }

    /// Borrow the underlying `sqlx` pool for consumer queries.
    ///
    /// Consumer SQL against this pool is supported and expected
    /// (SCOPE.md R4). Only *starter's* SQL is restricted to the
    /// store crates.
    pub fn sqlx(&self) -> &SqlitePool {
        &self.inner
    }
}
