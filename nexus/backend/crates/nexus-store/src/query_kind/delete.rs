//! Delete a query-kind.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete a query-kind by id within the tenant. An absent (or another tenant's)
/// id is a `NotFound`.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let result = sqlx::query("DELETE FROM nexus_query_kinds WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    if result.rows_affected() == 0 {
        return Err(Error::NotFound {
            what: "query-kind".into(),
        });
    }
    Ok(())
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
