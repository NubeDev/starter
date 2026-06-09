//! Panel persistence — create, list-by-dashboard, and delete. Panels carry the
//! tenant explicitly so RLS isolates them even though they hang off a dashboard.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewPanel, PanelPatch, PanelRecord};
use crate::tenant_tx;

/// Add a panel under its dashboard. The dashboard's tenant is bound by the
/// caller; the panel inherits it, so a panel can't be attached across tenants.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewPanel) -> Result<PanelRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_panels \
         (tenant_id, dashboard_id, datasource_id, title, sql, viz, layout) \
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING id",
    )
    .bind(tenant_id)
    .bind(new.dashboard_id)
    .bind(new.datasource_id)
    .bind(&new.title)
    .bind(&new.sql)
    .bind(&new.viz)
    .bind(&new.layout)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(PanelRecord {
        id: row.get::<Uuid, _>("id"),
        dashboard_id: new.dashboard_id,
        datasource_id: new.datasource_id,
        title: new.title.clone(),
        sql: new.sql.clone(),
        viz: new.viz.clone(),
        layout: new.layout.clone(),
    })
}

/// List the panels of one dashboard, oldest first (canvas order).
pub async fn list_for_dashboard(
    pool: &PgPool,
    tenant_id: &str,
    dashboard_id: Uuid,
) -> Result<Vec<PanelRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, dashboard_id, datasource_id, title, sql, viz, layout \
         FROM nexus_panels WHERE dashboard_id = $1 ORDER BY created_at",
    )
    .bind(dashboard_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// The dashboard a panel belongs to, within the tenant. `Ok(None)` when no such
/// panel is visible — used to authorize a panel mutation against its owning
/// dashboard's grant before the delete runs.
pub async fn dashboard_id_of(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<Uuid>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query("SELECT dashboard_id FROM nexus_panels WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.map(|r| r.get::<Uuid, _>("dashboard_id")))
}

/// Partial update of a panel within the tenant. Each `None` field is left
/// unchanged via COALESCE, so one statement handles any subset without dynamic
/// SQL. Returns the updated record, or `None` when no such panel is visible
/// (RLS hid it, or it does not exist). The owning `dashboard_id` is immutable —
/// a panel does not move between dashboards through this path.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &PanelPatch,
) -> Result<Option<PanelRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // `datasource_id` is nullable on the column, so a `None` in the patch means
    // "leave unchanged" rather than "set NULL". COALESCE gives exactly that —
    // clearing a datasource is not expressible here, which matches the DTO (the
    // UI only ever sets a datasource, never unsets it).
    let row = sqlx::query(
        "UPDATE nexus_panels SET \
           title         = COALESCE($2, title), \
           datasource_id = COALESCE($3, datasource_id), \
           sql           = COALESCE($4, sql), \
           viz           = COALESCE($5, viz), \
           layout        = COALESCE($6, layout) \
         WHERE id = $1 \
         RETURNING id, dashboard_id, datasource_id, title, sql, viz, layout",
    )
    .bind(id)
    .bind(&patch.title)
    .bind(patch.datasource_id)
    .bind(&patch.sql)
    .bind(&patch.viz)
    .bind(&patch.layout)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

/// Delete a panel within the tenant. Returns whether a row was removed.
pub async fn delete(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<bool, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let done = sqlx::query("DELETE FROM nexus_panels WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(done.rows_affected() > 0)
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> PanelRecord {
    PanelRecord {
        id: row.get::<Uuid, _>("id"),
        dashboard_id: row.get::<Uuid, _>("dashboard_id"),
        datasource_id: row.get::<Option<Uuid>, _>("datasource_id"),
        title: row.get::<String, _>("title"),
        sql: row.get::<String, _>("sql"),
        viz: row.get::<String, _>("viz"),
        layout: row.get::<serde_json::Value, _>("layout"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
