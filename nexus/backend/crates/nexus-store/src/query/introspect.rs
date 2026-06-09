//! Introspect a datasource's tables and columns for editor autocomplete.
//!
//! Unlike [`super::run_query`], the SQL here is fixed and server-owned: a single
//! read of `information_schema.columns` restricted to user schemas. It runs in
//! the same `READ ONLY` transaction with a `statement_timeout` so an unreachable
//! or slow catalog can't hang the request, and it returns a compact
//! table → columns map — never row data.

use futures::TryStreamExt;
use sqlx::{Executor, PgPool, Row};
use starter_spi::Error;

use super::QueryGuards;

/// One column of a table: its name and its declared SQL type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
}

/// One table (or view) and its columns, qualified by schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableInfo {
    pub schema: String,
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

// User-facing schemas only: Postgres' own catalogs (`pg_catalog`,
// `information_schema`, the `pg_toast*` schemas) are noise in an autocomplete
// and large, so they are excluded at the source rather than filtered client-side.
const INTROSPECT_SQL: &str = "\
    SELECT table_schema, table_name, column_name, data_type, ordinal_position \
    FROM information_schema.columns \
    WHERE table_schema NOT IN ('pg_catalog', 'information_schema') \
      AND table_schema NOT LIKE 'pg_toast%' \
      AND table_schema NOT LIKE 'pg_temp%' \
    ORDER BY table_schema, table_name, ordinal_position";

/// Introspect `pool`'s tables and columns. The `statement_timeout` guard bounds
/// wall-clock; `max_rows`/`max_bytes` are not applied — the catalog read is
/// already bounded by the schema filter, and a column list must not truncate
/// mid-table or autocomplete would silently lose fields.
pub async fn introspect(pool: &PgPool, guards: QueryGuards) -> Result<Vec<TableInfo>, Error> {
    let mut tx = pool.begin().await.map_err(internal)?;
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(internal)?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    tx.execute(format!("SET LOCAL statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(internal)?;

    let mut rows = sqlx::query(INTROSPECT_SQL).fetch(&mut *tx);
    // The query orders by (schema, table, ordinal), so columns of one table
    // arrive contiguously — group by appending to the last table when its key
    // matches, starting a new one otherwise.
    let mut tables: Vec<TableInfo> = Vec::new();
    while let Some(row) = rows.try_next().await.map_err(invalid)? {
        let schema: String = row.get("table_schema");
        let name: String = row.get("table_name");
        let column = ColumnInfo {
            name: row.get("column_name"),
            data_type: row.get("data_type"),
        };
        match tables.last_mut() {
            Some(t) if t.schema == schema && t.name == name => t.columns.push(column),
            _ => tables.push(TableInfo {
                schema,
                name,
                columns: vec![column],
            }),
        }
    }
    drop(rows);
    tx.commit().await.map_err(internal)?;
    Ok(tables)
}

/// A catalog read that errors mid-query is the datasource's fault to surface as
/// a 4xx (e.g. permissions on `information_schema`), like a user query.
fn invalid(e: sqlx::Error) -> Error {
    Error::Invalid {
        message: e.to_string(),
    }
}

/// A connection/transaction failure is ours — 5xx.
fn internal(e: sqlx::Error) -> Error {
    Error::Internal {
        source: Box::new(e),
    }
}
