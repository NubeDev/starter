//! Apply a partial update to a flow, and the focused enabled-flag toggle the
//! start/stop routes use.

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use super::record::FlowPatch;
use crate::tenant_tx;

/// Apply `patch` to flow `id` within `tenant_id`. Only supplied fields change.
/// Returns whether a row was updated (a cross-tenant update is a no-op via RLS).
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &FlowPatch,
) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // COALESCE keeps the current value where the patch field is NULL, so one
    // statement handles any subset without dynamic SQL.
    let done = sqlx::query(
        "UPDATE nexus_flows SET \
           name     = COALESCE($2, name), \
           input    = COALESCE($3, input), \
           pipeline = COALESCE($4, pipeline), \
           output   = COALESCE($5, output), \
           enabled  = COALESCE($6, enabled) \
         WHERE id = $1",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.input)
    .bind(&patch.pipeline)
    .bind(&patch.output)
    .bind(patch.enabled)
    .execute(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

/// Set just the enabled flag — what the start/stop routes flip. Returns whether
/// a row matched.
pub async fn set_enabled(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    enabled: bool,
) -> Result<bool, Error> {
    update(
        pool,
        tenant_id,
        id,
        &FlowPatch {
            enabled: Some(enabled),
            ..Default::default()
        },
    )
    .await
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
