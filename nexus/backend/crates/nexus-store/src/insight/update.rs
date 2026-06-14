//! Update an insight within a tenant.

use serde_json::Value;
use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{InsightPatch, InsightRecord};
use crate::tenant_tx;

/// Partial update of insight `id` within `tenant_id`. Each field uses COALESCE so
/// an unset field is left unchanged. Returns the updated record, or `None` when
/// no row in the tenant matches the id.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &InsightPatch,
) -> Result<Option<InsightRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "UPDATE nexus_insights SET \
           name          = COALESCE($2, name), \
           script        = COALESCE($3, script), \
           params_schema = COALESCE($4, params_schema) \
         WHERE id = $1 \
         RETURNING id, tenant_id, name, script, params_schema",
    )
    .bind(id)
    .bind(&patch.name)
    .bind(&patch.script)
    .bind(&patch.params_schema)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(row.map(|r| InsightRecord {
        id: r.get::<Uuid, _>("id"),
        tenant_id: r.get::<String, _>("tenant_id"),
        name: r.get::<String, _>("name"),
        script: r.get::<String, _>("script"),
        params_schema: r.get::<Option<Value>, _>("params_schema"),
    }))
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
