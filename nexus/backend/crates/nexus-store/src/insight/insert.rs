//! Create an insight for a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{InsightRecord, NewInsight};
use crate::tenant_tx;

/// Insert an insight into the caller's tenant. RLS binds the row to the tenant;
/// the returned record carries the fresh id.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewInsight,
) -> Result<InsightRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_insights (tenant_id, name, script, params_schema) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.script)
    .bind(&new.params_schema)
    .fetch_one(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(InsightRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        script: new.script.clone(),
        params_schema: new.params_schema.clone(),
    })
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
