//! Delete a dashboard within a tenant (cascading to its panels).

use sqlx::PgPool;
use starter_spi::Error;
use uuid::Uuid;

use crate::nav_node;
use crate::tag::{self, EntityRef};
use crate::tenant_tx;

/// Delete dashboard `id` if it belongs to `tenant_id`. Panels cascade via the
/// foreign key; tags have no DB cascade (the tag table references entities
/// polymorphically), so they are swept here. Returns whether a row was removed.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_dashboards WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    let removed = done.rows_affected() > 0;
    if removed {
        // Sweep the dashboard's tags. A separate transaction is fine: a crash
        // between the two leaves orphan tags, not corruption — the reverse
        // lookup is keyed by id so they're inert, and a re-delete clears them.
        tag::delete_for_entity(
            pool,
            tenant_id,
            &EntityRef {
                entity_type: "dashboard".into(),
                entity_id: id.to_string(),
            },
        )
        .await?;
        // Sweep any nav nodes that mounted this page back to plain `group`
        // headers (WS-13): losing the page must not delete the navigation node,
        // only blank its target so the user can retarget it. Same separate-tx
        // rationale as the tag sweep above.
        nav_node::sweep_dashboard_targets(pool, tenant_id, id).await?;
    }
    Ok(removed)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
