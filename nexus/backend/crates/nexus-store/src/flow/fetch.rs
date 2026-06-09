//! List and get flows within a tenant, and the cross-tenant load the
//! FlowManager uses at startup to resume enabled flows.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::FlowRecord;
use crate::tenant_tx;

/// List the tenant's flows, newest first.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<FlowRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, input, pipeline, output, enabled \
         FROM nexus_flows ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one flow by id within the tenant. `Ok(None)` covers both absent and
/// another tenant's, so existence is not leaked.
pub async fn get(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<Option<FlowRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, input, pipeline, output, enabled \
         FROM nexus_flows WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> FlowRecord {
    FlowRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        input: row.get::<serde_json::Value, _>("input"),
        pipeline: row.get::<serde_json::Value, _>("pipeline"),
        output: row.get::<serde_json::Value, _>("output"),
        enabled: row.get::<bool, _>("enabled"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
