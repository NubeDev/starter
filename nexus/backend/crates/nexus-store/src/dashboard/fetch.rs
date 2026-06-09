//! Read dashboards for a tenant — list, and resolve a slug to its id + record.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::DashboardRecord;
use crate::tenant_tx;

/// List the tenant's dashboards, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<DashboardRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, slug, name, icon, accent, folder_id, starred \
         FROM nexus_dashboards ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Resolve a slug to its dashboard within the tenant. `Ok(None)` covers both
/// "absent" and "another tenant's" — existence is not leaked.
pub async fn by_slug(
    pool: &PgPool,
    tenant_id: &str,
    slug: &str,
) -> Result<Option<DashboardRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, slug, name, icon, accent, folder_id, starred \
         FROM nexus_dashboards WHERE slug = $1",
    )
    .bind(slug)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> DashboardRecord {
    DashboardRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        slug: row.get::<String, _>("slug"),
        name: row.get::<String, _>("name"),
        icon: row.get::<String, _>("icon"),
        accent: row.get::<String, _>("accent"),
        folder_id: row.get::<Option<Uuid>, _>("folder_id"),
        starred: row.get::<bool, _>("starred"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
