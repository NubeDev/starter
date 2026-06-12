//! Read insights for a tenant — list, and fetch one by id.

use serde_json::Value;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::InsightRecord;
use crate::tenant_tx;

/// List the tenant's insights by name.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<InsightRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, script, params_schema \
         FROM nexus_insights ORDER BY name",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one insight by id within the tenant. `Ok(None)` covers both "absent" and
/// "another tenant's" — existence is not leaked.
pub async fn by_id(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<InsightRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, script, params_schema \
         FROM nexus_insights WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> InsightRecord {
    InsightRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        script: row.get::<String, _>("script"),
        params_schema: row.get::<Option<Value>, _>("params_schema"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
