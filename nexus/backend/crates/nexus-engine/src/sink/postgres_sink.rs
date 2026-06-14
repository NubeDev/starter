//! The native `postgres` sink: insert each batch's rows into a table in a
//! datasource Postgres, with optional schema-driven table creation.
//!
//! On the [`Sink`] trait: each row is converted to a JSON object via the shared
//! Arrow→JSON bridge and inserted with bound parameters typed by the stream's
//! Arrow schema (see [`super::pg_insert`]), never string-concatenated. The pool
//! is opened lazily on the first write and closed on `close`. The connection
//! string comes from the flow config (the datasource secret, decrypted by the
//! caller at build time), never from a request.
//!
//! Table control (the flow author's "how is the table made and stored"):
//! - `create` (default true): `CREATE TABLE IF NOT EXISTS` derived from the
//!   incoming batch's Arrow schema on the first write — the schema is the single
//!   source of truth, set by `json_to_arrow` (declared or inferred). Set false to
//!   require a pre-existing table.
//! - `primary_key`: column names that form the table's primary key when it is
//!   created. Also the conflict target for `on_conflict`.
//! - `on_conflict`: `error` (default — a duplicate key fails the write),
//!   `nothing` (skip duplicates), or `upsert` (update the non-key columns).
//!
//! Because column types come from the Arrow schema, a declared `timestamp` field
//! becomes a real `timestamptz` column and binds as one — no `text` workaround.

use serde::Deserialize;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::{DataType, SchemaRef};

use super::pg_insert::{column_types, insert_row, ColumnTypes};
use crate::arrow_json::batch_to_rows;
use crate::core::{EngineError, EngineResult, Sink};

/// How the sink resolves a row whose key collides with an existing row.
#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OnConflict {
    /// A duplicate key fails the write (Postgres default behaviour).
    #[default]
    Error,
    /// Skip a row whose key already exists (`ON CONFLICT DO NOTHING`).
    Nothing,
    /// Update the non-key columns of an existing row (`ON CONFLICT … DO UPDATE`).
    Upsert,
}

#[derive(Debug, Clone, Deserialize)]
struct PostgresConfig {
    /// Connection string for the target Postgres.
    uri: String,
    /// Table the shaped rows are inserted into.
    table: String,
    /// Create the table from the stream schema on first write if it is missing.
    #[serde(default = "default_create")]
    create: bool,
    /// Columns that form the primary key when the table is created; also the
    /// conflict target for `on_conflict`.
    #[serde(default)]
    primary_key: Vec<String>,
    /// What to do when an inserted row's key collides with an existing row.
    #[serde(default)]
    on_conflict: OnConflict,
}

fn default_create() -> bool {
    true
}

/// Inserts batch rows into a datasource Postgres table, opening its pool on the
/// first write and creating the table from the stream schema if asked.
pub struct PostgresSink {
    uri: String,
    table: String,
    create: bool,
    primary_key: Vec<String>,
    on_conflict: OnConflict,
    pool: Option<PgPool>,
    /// Set once the table has been ensured (created or assumed present), so the
    /// DDL runs at most once per run.
    ensured: bool,
}

impl PostgresSink {
    /// Build from the node config, requiring `uri` and `table`. No connection is
    /// made here — building is pure setup; the pool opens on the first write.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: PostgresConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid postgres config: {e}")))?;
        Ok(Self {
            uri: config.uri,
            table: config.table,
            create: config.create,
            primary_key: config.primary_key,
            on_conflict: config.on_conflict,
            pool: None,
            ensured: false,
        })
    }

    /// Open the pool on demand, returning a reference to it. Bounded to two
    /// connections — a flow's sink is low-throughput and long-lived.
    async fn pool(&mut self) -> EngineResult<&PgPool> {
        if self.pool.is_none() {
            let pool = PgPoolOptions::new()
                .max_connections(2)
                .connect(&self.uri)
                .await
                .map_err(|e| EngineError::Sink(format!("postgres connect failed: {e}")))?;
            self.pool = Some(pool);
        }
        Ok(self.pool.as_ref().expect("pool just set"))
    }

    /// `CREATE TABLE IF NOT EXISTS` for `table` derived from the batch schema.
    /// Runs once per run; a no-op when `create` is false.
    async fn ensure_table(&mut self, schema: &SchemaRef) -> EngineResult<()> {
        if self.ensured || !self.create {
            self.ensured = true;
            return Ok(());
        }
        let ddl = create_table_sql(&self.table, schema, &self.primary_key);
        let pool = self.pool().await?.clone();
        sqlx::query(&ddl)
            .execute(&pool)
            .await
            .map_err(|e| EngineError::Sink(format!("postgres create table failed: {e}")))?;
        self.ensured = true;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Sink for PostgresSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let schema = batch.schema();
        self.ensure_table(&schema).await?;

        let types: ColumnTypes = column_types(&schema);
        let conflict = on_conflict_clause(self.on_conflict, &self.primary_key, &schema);
        let rows = batch_to_rows(batch).map_err(EngineError::Sink)?.rows;
        // Split the borrow: take the owned bits out before borrowing the pool
        // mutably, so both live across the await.
        let table = self.table.clone();
        let pool = self.pool().await?.clone();
        for row in &rows {
            let obj = row
                .as_object()
                .ok_or_else(|| EngineError::Sink("postgres sink expects object rows".into()))?;
            insert_row(&pool, &table, obj, &types, conflict.as_deref()).await?;
        }
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        if let Some(pool) = self.pool.take() {
            pool.close().await;
        }
        Ok(())
    }
}

/// Build the `ON CONFLICT …` clause for the configured policy, or `None` when the
/// default (error on duplicate) applies or no usable key exists.
fn on_conflict_clause(
    policy: OnConflict,
    primary_key: &[String],
    schema: &SchemaRef,
) -> Option<String> {
    if policy == OnConflict::Error || primary_key.is_empty() {
        return None;
    }
    let key_list = primary_key
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");
    match policy {
        OnConflict::Error => None,
        OnConflict::Nothing => Some(format!("ON CONFLICT ({key_list}) DO NOTHING")),
        OnConflict::Upsert => {
            // Update every non-key column from the proposed row (EXCLUDED).
            let key: std::collections::HashSet<&str> =
                primary_key.iter().map(String::as_str).collect();
            let sets = schema
                .fields()
                .iter()
                .map(|f| f.name().as_str())
                .filter(|n| !key.contains(n))
                .map(|n| format!("\"{n}\" = EXCLUDED.\"{n}\""))
                .collect::<Vec<_>>();
            if sets.is_empty() {
                // All columns are in the key — nothing to update, so skip.
                Some(format!("ON CONFLICT ({key_list}) DO NOTHING"))
            } else {
                Some(format!(
                    "ON CONFLICT ({key_list}) DO UPDATE SET {}",
                    sets.join(", ")
                ))
            }
        }
    }
}

/// `CREATE TABLE IF NOT EXISTS` with one column per Arrow field, mapping each
/// Arrow type to its Postgres column type, plus an optional primary key.
fn create_table_sql(table: &str, schema: &SchemaRef, primary_key: &[String]) -> String {
    let mut cols = Vec::with_capacity(schema.fields().len());
    for field in schema.fields() {
        let pg_type = pg_column_type(field.data_type());
        let null = if field.is_nullable() { "" } else { " NOT NULL" };
        cols.push(format!("\"{}\" {pg_type}{null}", field.name()));
    }
    if !primary_key.is_empty() {
        let key_list = primary_key
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        cols.push(format!("PRIMARY KEY ({key_list})"));
    }
    format!(
        "CREATE TABLE IF NOT EXISTS \"{table}\" ({})",
        cols.join(", ")
    )
}

/// Map an Arrow `DataType` to the Postgres column type the sink creates and binds
/// against. Kept coarse to match `json_to_arrow`'s closed declared-type set.
fn pg_column_type(dt: &DataType) -> &'static str {
    match dt {
        DataType::Boolean => "boolean",
        DataType::Int8
        | DataType::Int16
        | DataType::Int32
        | DataType::Int64
        | DataType::UInt8
        | DataType::UInt16
        | DataType::UInt32
        | DataType::UInt64 => "bigint",
        DataType::Float16 | DataType::Float32 | DataType::Float64 => "double precision",
        DataType::Timestamp(_, _) | DataType::Date32 | DataType::Date64 => "timestamptz",
        // Utf8/everything else: text. A nested shape round-trips as JSON text.
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use datafusion::arrow::datatypes::{Field, Schema, TimeUnit};
    use std::sync::Arc;

    fn schema() -> SchemaRef {
        Arc::new(Schema::new(vec![
            Field::new("site_id", DataType::Utf8, false),
            Field::new(
                "ts",
                DataType::Timestamp(TimeUnit::Microsecond, None),
                false,
            ),
            Field::new("value", DataType::Float64, true),
            Field::new("n", DataType::Int64, true),
            Field::new("ok", DataType::Boolean, true),
        ]))
    }

    #[test]
    fn create_table_maps_arrow_types_to_pg() {
        let sql = create_table_sql("telemetry", &schema(), &["site_id".into(), "ts".into()]);
        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS \"telemetry\" (\
\"site_id\" text NOT NULL, \"ts\" timestamptz NOT NULL, \
\"value\" double precision, \"n\" bigint, \"ok\" boolean, \
PRIMARY KEY (\"site_id\", \"ts\"))"
        );
    }

    #[test]
    fn create_table_without_key_omits_primary_key() {
        let sql = create_table_sql("t", &schema(), &[]);
        assert!(!sql.contains("PRIMARY KEY"));
        assert!(sql.starts_with("CREATE TABLE IF NOT EXISTS \"t\" ("));
    }

    #[test]
    fn on_conflict_error_or_no_key_is_none() {
        assert!(on_conflict_clause(OnConflict::Error, &["ts".into()], &schema()).is_none());
        assert!(on_conflict_clause(OnConflict::Nothing, &[], &schema()).is_none());
    }

    #[test]
    fn on_conflict_nothing_targets_the_key() {
        let c = on_conflict_clause(
            OnConflict::Nothing,
            &["site_id".into(), "ts".into()],
            &schema(),
        )
        .unwrap();
        assert_eq!(c, "ON CONFLICT (\"site_id\", \"ts\") DO NOTHING");
    }

    #[test]
    fn on_conflict_upsert_updates_non_key_columns() {
        let c = on_conflict_clause(
            OnConflict::Upsert,
            &["site_id".into(), "ts".into()],
            &schema(),
        )
        .unwrap();
        assert_eq!(
            c,
            "ON CONFLICT (\"site_id\", \"ts\") DO UPDATE SET \
\"value\" = EXCLUDED.\"value\", \"n\" = EXCLUDED.\"n\", \"ok\" = EXCLUDED.\"ok\""
        );
    }

    #[test]
    fn config_defaults_create_true_error_conflict() {
        let cfg: PostgresConfig =
            serde_json::from_value(serde_json::json!({"uri":"u","table":"t"})).unwrap();
        assert!(cfg.create);
        assert_eq!(cfg.on_conflict, OnConflict::Error);
        assert!(cfg.primary_key.is_empty());
    }
}
