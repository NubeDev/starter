//! Delete an agent (its sessions cascade via the FK).

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete an agent by id within the tenant. Its sessions are removed by the
/// `ON DELETE CASCADE` FK. Returns whether a row matched.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let result = sqlx::query("DELETE FROM nexus_agents WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(result.rows_affected() > 0)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
