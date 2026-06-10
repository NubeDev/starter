//! Read folders for a tenant — list, and fetch one by id.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::FolderRecord;
use crate::tenant_tx;

/// List the tenant's folders, parent-first then by name, so the caller can
/// build the tree top-down without re-sorting.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<FolderRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, parent_id, name \
         FROM nexus_folders ORDER BY parent_id NULLS FIRST, name",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one folder by id within the tenant. `Ok(None)` covers both "absent" and
/// "another tenant's" — existence is not leaked.
pub async fn by_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<FolderRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, parent_id, name FROM nexus_folders WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> FolderRecord {
    FolderRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        parent_id: row.get::<Option<Uuid>, _>("parent_id"),
        name: row.get::<String, _>("name"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
