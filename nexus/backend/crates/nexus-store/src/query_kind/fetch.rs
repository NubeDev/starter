//! List and get query-kinds within a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::QueryKindRecord;
use crate::tenant_tx;

/// List the tenant's query-kinds, name-ordered for the picker.
pub async fn list(pool: &PgPool, tenant_id: &str) -> Result<Vec<QueryKindRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let rows = sqlx::query(
        "SELECT id, tenant_id, name, sql, params_schema, datasource_kind, tables, \
                datasource_binding, description \
         FROM nexus_query_kinds ORDER BY name",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(rows.iter().map(row_to_kind).collect())
}

/// Fetch one query-kind by id within the tenant. Absent (or another tenant's,
/// which RLS hides) is a `NotFound`.
pub async fn get(pool: &PgPool, tenant_id: &str, id: Uuid) -> Result<QueryKindRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, sql, params_schema, datasource_kind, tables, \
                datasource_binding, description \
         FROM nexus_query_kinds WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    row.as_ref().map(row_to_kind).ok_or(Error::NotFound {
        what: "query-kind".into(),
    })
}

/// Fetch one query-kind by name within the tenant. `Ok(None)` when no row
/// matches — the dispatcher calls this on a registry miss and treats a missing
/// kind as "fall through", not an error.
pub async fn get_by_name(
    pool: &PgPool,
    tenant_id: &str,
    name: &str,
) -> Result<Option<QueryKindRecord>, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "SELECT id, tenant_id, name, sql, params_schema, datasource_kind, tables, \
                datasource_binding, description \
         FROM nexus_query_kinds WHERE name = $1",
    )
    .bind(name)
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;
    Ok(row.as_ref().map(row_to_kind))
}

fn row_to_kind(row: &sqlx::postgres::PgRow) -> QueryKindRecord {
    QueryKindRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: row.get::<String, _>("tenant_id"),
        name: row.get::<String, _>("name"),
        sql: row.get::<String, _>("sql"),
        params_schema: row.get::<serde_json::Value, _>("params_schema"),
        datasource_kind: row.get::<String, _>("datasource_kind"),
        tables: row.get::<Vec<String>, _>("tables"),
        datasource_binding: row.get::<Option<String>, _>("datasource_binding"),
        description: row.get::<Option<String>, _>("description"),
    }
}

fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
