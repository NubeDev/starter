//! Create a dashboard for a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{DashboardRecord, NewDashboard};
use crate::tenant_tx;

/// Insert a dashboard. A slug already used in the tenant is a `Conflict`, not a
/// silent alias. The id is minted by the DB; folder/star default at the column.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewDashboard,
) -> Result<DashboardRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_dashboards (tenant_id, slug, name, icon, accent, folder_id) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.slug)
    .bind(&new.name)
    .bind(&new.icon)
    .bind(&new.accent)
    .bind(new.folder_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(DashboardRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        slug: new.slug.clone(),
        name: new.name.clone(),
        icon: new.icon.clone(),
        accent: new.accent.clone(),
        folder_id: new.folder_id,
        starred: false,
    })
}

/// Insert a dashboard with a caller-supplied id. Unlike [`insert`], the id is
/// not minted by the DB — this is the **id-stable** path WS-12's undo-of-delete /
/// redo-of-create needs: resurrecting a deleted dashboard must re-create it under
/// its *original* id so panels and grants keyed on that id stay valid. A slug
/// already used in the tenant is a `Conflict`; a duplicate id is a `Conflict`
/// too (the PK is the original row's id, which is the intended target).
pub async fn insert_with_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    new: &NewDashboard,
) -> Result<DashboardRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    sqlx::query(
        "INSERT INTO nexus_dashboards (id, tenant_id, slug, name, icon, accent, folder_id) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(&new.slug)
    .bind(&new.name)
    .bind(&new.icon)
    .bind(&new.accent)
    .bind(new.folder_id)
    .execute(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(DashboardRecord {
        id,
        tenant_id: tenant_id.to_string(),
        slug: new.slug.clone(),
        name: new.name.clone(),
        icon: new.icon.clone(),
        accent: new.accent.clone(),
        folder_id: new.folder_id,
        starred: false,
    })
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
