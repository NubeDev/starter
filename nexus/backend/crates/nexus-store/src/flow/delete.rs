//! Delete a flow within a tenant.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete flow `id` if it belongs to `tenant_id`. Returns whether a row was
/// removed; a cross-tenant delete is a silent no-op via RLS.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_flows WHERE id = $1")
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
