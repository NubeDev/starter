//! [`Prune`] impl over `starter_changes` (Postgres).

use async_trait::async_trait;
use starter_changelog::{Prune, PruneReport, PruneRequest};
use starter_spi::{Error, Result};
use starter_store_postgres::Pool;

/// Postgres-backed [`Prune`].
#[derive(Clone)]
pub struct PgChangePrune {
    pool: Pool,
}

impl PgChangePrune {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Prune for PgChangePrune {
    async fn prune(&self, req: &PruneRequest) -> Result<PruneReport> {
        if req.dry_run {
            let n: i64 = if let Some(kind) = &req.resource_kind {
                sqlx::query_scalar(
                    "SELECT COUNT(*) FROM starter_changes \
                     WHERE at < $1 AND resource_kind = $2",
                )
                .bind(req.before)
                .bind(kind.clone())
                .fetch_one(self.pool.sqlx())
                .await
            } else {
                sqlx::query_scalar("SELECT COUNT(*) FROM starter_changes WHERE at < $1")
                    .bind(req.before)
                    .fetch_one(self.pool.sqlx())
                    .await
            }
            .map_err(internal)?;
            return Ok(PruneReport { rows: n as u64 });
        }

        let result = if let Some(kind) = &req.resource_kind {
            sqlx::query(
                "DELETE FROM starter_changes WHERE at < $1 AND resource_kind = $2",
            )
            .bind(req.before)
            .bind(kind.clone())
            .execute(self.pool.sqlx())
            .await
        } else {
            sqlx::query("DELETE FROM starter_changes WHERE at < $1")
                .bind(req.before)
                .execute(self.pool.sqlx())
                .await
        };

        Ok(PruneReport {
            rows: result.map_err(internal)?.rows_affected(),
        })
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
