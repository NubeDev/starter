//! Rename / re-slug a dashboard within a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DashboardPatch, DashboardRecord};
use crate::tenant_tx;

/// Partial update of dashboard `id` within `tenant_id`. Each `None` field is
/// left unchanged via COALESCE, so one statement handles any subset without
/// dynamic SQL. The immutable id is never touched — re-slugging only changes
/// the route alias, so grants and panel refs (keyed on id) keep pointing at the
/// same dashboard. Returns the updated record, or `None` when no such dashboard
/// is visible (RLS hid it, or it does not exist). A slug already used in the
/// tenant is a `Conflict`, mirroring [`insert`](super::insert).
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &DashboardPatch,
) -> Result<Option<DashboardRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "UPDATE nexus_dashboards SET \
           name   = COALESCE($2, name), \
           slug   = COALESCE($3, slug), \
           icon   = COALESCE($4, icon), \
           accent = COALESCE($5, accent) \
         WHERE id = $1 \
         RETURNING id, tenant_id, slug, name, icon, accent",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.slug)
    .bind(&patch.icon)
    .bind(&patch.accent)
    .fetch_optional(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.map(|r| DashboardRecord {
        id: r.get::<Uuid, _>("id"),
        tenant_id: r.get::<String, _>("tenant_id"),
        slug: r.get::<String, _>("slug"),
        name: r.get::<String, _>("name"),
        icon: r.get::<String, _>("icon"),
        accent: r.get::<String, _>("accent"),
    }))
}

/// A unique-violation on (tenant_id, slug) is the caller's conflict; anything
/// else is ours.
fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a dashboard with that slug already exists".into(),
            };
        }
    }
    internal(e)
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
