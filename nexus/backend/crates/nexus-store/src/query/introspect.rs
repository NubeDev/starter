//! Introspect a datasource's tables, columns, and foreign keys for the editor's
//! autocomplete and the schema (ER) diagram.
//!
//! Unlike [`super::run_query`], the SQL here is fixed and server-owned: a read of
//! `information_schema.columns` plus a join over the FK catalog views, both
//! restricted to user schemas. It runs in a `READ ONLY` transaction with a
//! `statement_timeout` so an unreachable or slow catalog can't hang the request,
//! and it returns a compact table → columns map plus the table→table relations —
//! never row data.

use futures::TryStreamExt;
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};
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

/// One foreign-key edge: a column on `from_*` references a column on `to_*`.
/// Schema-qualified on both ends so the diagram can resolve the exact tables
/// even when a name is reused across schemas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationInfo {
    pub from_schema: String,
    pub from_table: String,
    pub from_column: String,
    pub to_schema: String,
    pub to_table: String,
    pub to_column: String,
}

/// The full introspection result: tables (with columns) and the FK edges
/// between them. Returned together so a single round-trip feeds both the
/// autocomplete tree and the ER diagram.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchemaInfo {
    pub tables: Vec<TableInfo>,
    pub relations: Vec<RelationInfo>,
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

// Foreign keys across user schemas. `information_schema` exposes FKs as a chain:
// `table_constraints` (the FK constraint) → `key_column_usage` (the referencing
// column) → `constraint_column_usage` (the referenced column). Joining the three
// on the constraint name yields one row per (from_col → to_col) pair, which is
// exactly one ER edge. Same user-schema filter as the column read so the two
// halves stay consistent.
const FK_SQL: &str = "\
    SELECT \
      tc.table_schema      AS from_schema, \
      tc.table_name        AS from_table, \
      kcu.column_name      AS from_column, \
      ccu.table_schema     AS to_schema, \
      ccu.table_name       AS to_table, \
      ccu.column_name      AS to_column \
    FROM information_schema.table_constraints tc \
    JOIN information_schema.key_column_usage kcu \
      ON kcu.constraint_name = tc.constraint_name \
     AND kcu.constraint_schema = tc.constraint_schema \
    JOIN information_schema.constraint_column_usage ccu \
      ON ccu.constraint_name = tc.constraint_name \
     AND ccu.constraint_schema = tc.constraint_schema \
    WHERE tc.constraint_type = 'FOREIGN KEY' \
      AND tc.table_schema NOT IN ('pg_catalog', 'information_schema') \
      AND tc.table_schema NOT LIKE 'pg_toast%' \
      AND tc.table_schema NOT LIKE 'pg_temp%' \
    ORDER BY from_schema, from_table, from_column";

/// Introspect `pool`'s tables, columns, and foreign keys. The `statement_timeout`
/// guard bounds wall-clock; `max_rows`/`max_bytes` are not applied — the catalog
/// read is already bounded by the schema filter, and a column list must not
/// truncate mid-table or autocomplete would silently lose fields.
pub async fn introspect(pool: &PgPool, guards: QueryGuards) -> Result<SchemaInfo, Error> {
    let mut tx = pool.begin().await.map_err(internal)?;
    begin_ro(&mut tx, guards).await?;
    let schema = read_schema(&mut tx).await?;
    tx.commit().await.map_err(internal)?;
    Ok(schema)
}

/// Like [`introspect`], but binds `app.tenant_id` for the transaction so RLS
/// scopes the catalog read to one tenant — the metadata-pool path used by the
/// admin nexus-DB schema inspector. `information_schema` itself is not under
/// RLS, so the tenant binding does not change the *shape* returned here; it is
/// set for parity with the tenant-scoped query path and so the same read-only
/// guarantees hold inside one bound transaction.
pub async fn introspect_tenant_ro(
    pool: &PgPool,
    tenant_id: &str,
    guards: QueryGuards,
) -> Result<SchemaInfo, Error> {
    let mut tx = pool.begin().await.map_err(internal)?;
    begin_ro(&mut tx, guards).await?;
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;
    let schema = read_schema(&mut tx).await?;
    tx.commit().await.map_err(internal)?;
    Ok(schema)
}

/// Set the transaction read-only and bound on wall-clock — shared by both the
/// datasource and tenant-scoped paths so neither can write or hang.
async fn begin_ro(tx: &mut Transaction<'_, Postgres>, guards: QueryGuards) -> Result<(), Error> {
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(internal)?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    tx.execute(format!("SET LOCAL statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(internal)?;
    Ok(())
}

/// Run the two catalog reads (columns, then FKs) inside an already-prepared
/// transaction and assemble the `SchemaInfo`.
async fn read_schema(tx: &mut Transaction<'_, Postgres>) -> Result<SchemaInfo, Error> {
    // Columns. The query orders by (schema, table, ordinal), so columns of one
    // table arrive contiguously — group by appending to the last table when its
    // key matches, starting a new one otherwise.
    let mut tables: Vec<TableInfo> = Vec::new();
    {
        let mut rows = sqlx::query(INTROSPECT_SQL).fetch(&mut **tx);
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
    }

    // Foreign keys. One row per referencing→referenced column pair.
    let mut relations: Vec<RelationInfo> = Vec::new();
    {
        let mut rows = sqlx::query(FK_SQL).fetch(&mut **tx);
        while let Some(row) = rows.try_next().await.map_err(invalid)? {
            relations.push(RelationInfo {
                from_schema: row.get("from_schema"),
                from_table: row.get("from_table"),
                from_column: row.get("from_column"),
                to_schema: row.get("to_schema"),
                to_table: row.get("to_table"),
                to_column: row.get("to_column"),
            });
        }
    }

    Ok(SchemaInfo { tables, relations })
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
