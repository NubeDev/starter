//! Update query-kinds.

use sqlx::{PgPool, Row};
use starter_spi::Error;
use uuid::Uuid;

use super::record::{QueryKindPatch, QueryKindRecord};
use crate::tenant_tx;

/// Apply a partial update to a query-kind and return the updated row. `None`
/// fields are left unchanged; `datasource_binding`/`description` set to
/// `Some(None)` clear the column. `name` is immutable — a kind is not renamed.
/// An absent (or another tenant's) id is a `NotFound`.
pub async fn update(
    pool: &PgPool,
    tenant_id: &str,
    id: Uuid,
    patch: &QueryKindPatch,
) -> Result<QueryKindRecord, Error> {
    let mut tx = tenant_tx::begin(pool, tenant_id).await?;
    // COALESCE keeps the existing value when the bound parameter is NULL, except
    // datasource_binding/description which are set directly so they can be
    // cleared to NULL. RETURNING gives back the updated row in one round-trip.
    let row = sqlx::query(
        "UPDATE nexus_query_kinds SET \
            sql                = COALESCE($2, sql), \
            params_schema      = COALESCE($3, params_schema), \
            datasource_kind    = COALESCE($4, datasource_kind), \
            tables             = COALESCE($5, tables), \
            datasource_binding = CASE WHEN $6 THEN $7 ELSE datasource_binding END, \
            description        = CASE WHEN $8 THEN $9 ELSE description END \
         WHERE id = $1 \
         RETURNING id, tenant_id, name, sql, params_schema, datasource_kind, tables, \
                   datasource_binding, description",
    )
    .bind(id)
    .bind(patch.sql.as_ref())
    .bind(patch.params_schema.as_ref())
    .bind(patch.datasource_kind.as_ref())
    .bind(patch.tables.as_ref())
    .bind(patch.datasource_binding.is_some())
    .bind(patch.datasource_binding.as_ref().and_then(|o| o.as_ref()))
    .bind(patch.description.is_some())
    .bind(patch.description.as_ref().and_then(|o| o.as_ref()))
    .fetch_optional(&mut *tx)
    .await
    .map_err(internal)?;
    tx.commit().await.map_err(internal)?;

    row.as_ref().map(row_to_kind).ok_or(Error::NotFound {
        what: "query-kind".into(),
    })
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
