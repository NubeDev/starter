//! Insert query-kinds owned by a tenant.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{NewQueryKind, QueryKindRecord};
use crate::tenant_tx;

/// Insert a new query-kind. A name already used in the tenant is a `Conflict`,
/// mirror of the flow-name rule.
pub async fn insert(
    pool: &PgPool,
    tenant_id: &str,
    new: &NewQueryKind,
) -> Result<QueryKindRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    let row = sqlx::query(
        "INSERT INTO nexus_query_kinds \
            (tenant_id, name, sql, params_schema, datasource_kind, tables, datasource_binding, description) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id",
    )
    .bind(tenant_id)
    .bind(&new.name)
    .bind(&new.sql)
    .bind(&new.params_schema)
    .bind(&new.datasource_kind)
    .bind(&new.tables)
    .bind(&new.datasource_binding)
    .bind(&new.description)
    .fetch_one(&mut *tx)
    .await
    .map_err(conflict_or_internal)?;
    tx.commit().await.map_err(internal)?;

    Ok(QueryKindRecord {
        id: row.get::<Uuid, _>("id"),
        tenant_id: tenant_id.to_string(),
        name: new.name.clone(),
        sql: new.sql.clone(),
        params_schema: new.params_schema.clone(),
        datasource_kind: new.datasource_kind.clone(),
        tables: new.tables.clone(),
        datasource_binding: new.datasource_binding.clone(),
        description: new.description.clone(),
    })
}

fn conflict_or_internal(e: sqlx::Error) -> Error {
    if let sqlx::Error::Database(db) = &e {
        if db.is_unique_violation() {
            return Error::Conflict {
                message: "a query-kind with that name already exists".into(),
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
