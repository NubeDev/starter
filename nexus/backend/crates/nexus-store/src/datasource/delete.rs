//! Delete a datasource within a tenant.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete datasource `id` if it belongs to `tenant_id`. Returns whether a row
/// was removed; RLS makes a cross-tenant delete a no-op rather than an error.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_datasources WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })?;
    tx.commit().await.map_err(|e| Error::Internal {
        source: Box::new(e),
    })?;
    Ok(done.rows_affected() > 0)
}
