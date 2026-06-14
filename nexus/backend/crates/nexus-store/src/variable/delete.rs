//! Delete a dashboard variable.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete variable `id` within the tenant. Returns whether a row matched, so the
/// route can answer 404 for an absent / other-tenant id without leaking which.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_dashboard_variables WHERE id = $1")
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
