//! Pin or unpin a history row as a favourite.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::internal;
use crate::tenant_tx;

/// Set the `starred` flag on one history row owned by `(tenant, user)`. Returns
/// whether a row was updated — `false` means the id was not this user's (RLS
/// and the `user_id` filter together scope the write). A starred row is exempt
/// from retention pruning.
pub async fn set_starred(
    pool: &PgPool,
    tenant_id: &str,
    user_id: &str,
    id: Uuid,
    starred: bool,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done =
        sqlx::query("UPDATE nexus_query_history SET starred = $1 WHERE id = $2 AND user_id = $3")
            .bind(starred)
            .bind(id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}
