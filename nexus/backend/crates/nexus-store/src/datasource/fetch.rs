//! Read datasources for a tenant — list and get-by-id. Neither path touches the
//! secret columns; the redacted record is all a reader needs.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::DatasourceRecord;
use crate::tenant_tx;

/// List the tenant's datasources (id/name/kind/connection, no secret), newest
/// first. RLS restricts the rows to the bound tenant.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<DatasourceRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, kind, host, port, database, db_user, key_version, config \
         FROM nexus_datasources ORDER BY created_at DESC",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_record).collect())
}

/// Fetch one datasource by id within the tenant. `Ok(None)` when no such row is
/// visible — which covers both "absent" and "another tenant's" identically, so
/// existence is not leaked across tenants.
pub async fn get(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
) -> Result<Option<DatasourceRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, kind, host, port, database, db_user, key_version, config \
         FROM nexus_datasources WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_record))
}

fn row_to_record(row: &sqlx::postgres::PgRow) -> DatasourceRecord {
    // The connection columns are nullable since file kinds populate none of them;
    // a missing column reads back as the type default so the record stays a plain
    // value, with the kind-specific shape carried in `config`.
    DatasourceRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        kind: row.get::<String, _>("kind"),
        host: row.get::<Option<String>, _>("host").unwrap_or_default(),
        port: row.get::<Option<i32>, _>("port").unwrap_or_default(),
        database: row.get::<Option<String>, _>("database").unwrap_or_default(),
        db_user: row.get::<Option<String>, _>("db_user").unwrap_or_default(),
        key_version: row.get::<i32, _>("key_version"),
        config: row.get::<Option<serde_json::Value>, _>("config"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
