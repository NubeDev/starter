//! List and get nav nodes within a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::NavNodeRecord;
use crate::tenant_tx;

const COLUMNS: &str =
    "id, tenant_id, parent_id, title, sort_order, target, context, icon, accent";

/// List the tenant's nav nodes, parent-first then by sort order, so the caller
/// can build the tree top-down without re-sorting.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<NavNodeRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM nexus_nav_nodes \
         ORDER BY parent_id NULLS FIRST, sort_order, title"
    ))
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one nav node by id within the tenant. `Ok(None)` covers both "absent"
/// and "another tenant's" — existence is not leaked.
pub async fn by_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<NavNodeRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(&format!(
        "SELECT {COLUMNS} FROM nexus_nav_nodes WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

pub(super) fn row_to_record(row: &sqlx::postgres::PgRow) -> NavNodeRecord {
    NavNodeRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        parent_id: row.get::<Option<Uuid>, _>("parent_id"),
        title: row.get::<String, _>("title"),
        sort_order: row.get::<i32, _>("sort_order"),
        target: row.get::<serde_json::Value, _>("target"),
        context: row.get::<Option<serde_json::Value>, _>("context"),
        icon: row.get::<Option<String>, _>("icon"),
        accent: row.get::<Option<String>, _>("accent"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
