//! Insert a flow owned by a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{FlowRecord, NewFlow};
use crate::tenant_tx;

/// Insert a new flow. A name already used in the tenant is a `Conflict`, mirror
/// of the dashboard-slug rule.
pub async fn insert(pool: &PgPool, tenant_id: &str, new: &NewFlow) -> Result<FlowRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_flows (tenant_id, name, input, pipeline, output, enabled) \
         VALUES ($1,$2,$3,$4,$5,$6) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.input)
    .bind(&new.pipeline)
    .bind(&new.output)
    .bind(new.enabled)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(FlowRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        input: new.input.clone(),
        pipeline: new.pipeline.clone(),
        output: new.output.clone(),
        enabled: new.enabled,
    })
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a flow with that name already exists".into(),
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
