//! Delete an insight within a tenant.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete insight `id` if it belongs to `tenant_id`. Returns whether a row was
/// removed. A panel that referenced it is left with a dangling id the query path
/// resolves to a clean "not found" rather than a hard failure.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_insights WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
