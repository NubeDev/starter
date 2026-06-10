//! Insert a query run and trim the user's history to the retention bound.
//!
//! Recording is best-effort from the caller's view but transactional here: the
//! insert and the per-user trim run in one tenant-bound transaction so a run is
//! never half-recorded and the ledger never grows past [`RETENTION_PER_USER`].

use sqlx::PgPool;
use starter_spi::Error;

use super::row::NewQueryRun;
use super::{internal, RETENTION_PER_USER};
use crate::tenant_tx;

/// Record one query run for `(tenant, user)` and prune older rows beyond the
/// retention bound. A starred row is never pruned — a user's pinned favourites
/// outlive the rolling window.
pub async fn record_run(
    pool: &PgPool,
    tenant_id: &str,
    run: &NewQueryRun,
) -> Result<(), Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_query_history \
         (tenant_id, user_id, datasource_id, sql, elapsed_ms, row_count, error) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(tenant_id)
    .bind(&run.user_id)
    .bind(run.datasource_id)
    .bind(&run.sql)
    .bind(run.elapsed_ms)
    .bind(run.row_count)
    .bind(&run.error)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;

    // Keep the newest `RETENTION_PER_USER` un-starred rows; starred rows are
    // exempt so a pinned query survives the rolling window. The subquery picks
    // the ids to keep, then the delete sweeps everything else for this user.
    sqlx::query(
        "DELETE FROM nexus_query_history \
         WHERE user_id = $1 AND NOT starred AND id NOT IN ( \
             SELECT id FROM nexus_query_history \
             WHERE user_id = $1 AND NOT starred \
             ORDER BY ran_at DESC LIMIT $2 \
         )",
    )
    .bind(&run.user_id)
    .bind(RETENTION_PER_USER)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;

    tx.commit().await.map_err(internal)?;
    Ok(())
}
