//! Delete a nav node, and sweep dashboard targets when a page is deleted.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::tenant_tx;

/// Delete nav node `id` if it belongs to `tenant_id`. Child nodes are
/// **re-rooted**, not deleted: the `ON DELETE SET NULL` self-reference means
/// removing a branch header never destroys the nodes filed under it. Returns
/// whether a row was removed.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_nav_nodes WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Reset every nav node that targets dashboard `dashboard_id` back to a plain
/// `{ "kind": "group" }` header, clearing its now-dangling context. Called from
/// the dashboard delete path (same place tags are swept): losing the page must
/// not delete the nav node — the user keeps their navigation structure, the
/// mount just becomes an empty header to retarget. Returns the count swept.
///
/// The match is on the JSONB `target ->> 'dashboardId'`, so only dashboard-kind
/// targets pointing at this id are affected; group/route nodes are untouched.
pub async fn sweep_dashboard_targets(
    pool: &PgPool,
    tenant_id: &str,
    dashboard_id: Uuid,
) -> Result<u64, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query(
        "UPDATE nexus_nav_nodes \
         SET target = '{\"kind\":\"group\"}'::jsonb, context = NULL \
         WHERE target ->> 'kind' = 'dashboard' \
           AND target ->> 'dashboardId' = $1",
    )
    .bind(dashboard_id.to_string())
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected())
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
