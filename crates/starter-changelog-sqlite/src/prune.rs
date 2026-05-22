//! [`Prune`] impl over `starter_changes`.

use async_trait::async_trait;
use starter_changelog::{Prune, PruneReport, PruneRequest};
use starter_spi::{Error, Result};
use starter_store_sqlite::Pool;

/// SQLite-backed [`Prune`].
#[derive(Clone)]
pub struct SqliteChangePrune {
    pool: Pool,
}

impl SqliteChangePrune {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Prune for SqliteChangePrune {
    async fn prune(&self, req: &PruneRequest) -> Result<PruneReport> {
        let before_text = req.before.to_rfc3339();
        if req.dry_run {
            let mut q = sqlx::query_scalar::<_, i64>(if req.resource_kind.is_some() {
                "SELECT COUNT(*) FROM starter_changes WHERE at < ?1 AND resource_kind = ?2"
            } else {
                "SELECT COUNT(*) FROM starter_changes WHERE at < ?1"
            })
            .bind(&before_text);
            if let Some(kind) = &req.resource_kind {
                q = q.bind(kind.clone());
            }
            let n = q.fetch_one(self.pool.sqlx()).await.map_err(internal)?;
            return Ok(PruneReport { rows: n as u64 });
        }

        let result = if let Some(kind) = &req.resource_kind {
            sqlx::query("DELETE FROM starter_changes WHERE at < ?1 AND resource_kind = ?2")
                .bind(&before_text)
                .bind(kind.clone())
                .execute(self.pool.sqlx())
                .await
        } else {
            sqlx::query("DELETE FROM starter_changes WHERE at < ?1")
                .bind(&before_text)
                .execute(self.pool.sqlx())
                .await
        };

        let rows = result.map_err(internal)?.rows_affected();
        Ok(PruneReport { rows })
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
