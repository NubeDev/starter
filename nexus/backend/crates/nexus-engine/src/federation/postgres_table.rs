//! A DataFusion `TableProvider` that reads a remote Postgres table over sqlx.
//!
//! This is the hand-written provider RW-05's session log records adopting over
//! `datafusion-table-providers`: that crate's Postgres provider is built on
//! `tokio-postgres` + `bb8`, a second Postgres client stack roadmap §8 forbids
//! (RW-04 chose sqlx for the same reason). Here the provider reuses the sqlx
//! stack the rest of nexus uses.
//!
//! The provider pulls each row as a JSON object (`to_jsonb`) and lets Arrow infer
//! the schema (see [`super::rows_to_batch`]), so it needs no per-Postgres-type
//! mapping table. On `scan` it fetches once into a `MemTable` and delegates, so
//! DataFusion applies projection/filter/limit above the materialised batch.
//!
//! Input-side memory bound (roadmap §6, this spec): the fetch is capped by
//! `max_fetch_rows` with a `LIMIT`, so a join cannot pull an unbounded remote
//! table into memory before the output caps ever apply. The table name is
//! validated as a strict identifier before it reaches SQL text.

use std::any::Any;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::catalog::{Session, TableProvider};
use datafusion::datasource::MemTable;
use datafusion::error::{DataFusionError, Result as DfResult};
use datafusion::logical_expr::{Expr, TableType};
use datafusion::physical_plan::ExecutionPlan;
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{PgPool, Row};

use super::identifier::validate_identifier;
use super::rows_to_batch::json_rows_to_batch;
use super::source::PostgresConn;
use crate::core::{EngineError, EngineResult};

/// Reads one remote Postgres table into the federation engine on demand.
#[derive(Debug)]
pub struct PostgresTableProvider {
    pool: PgPool,
    table: String,
    max_fetch_rows: usize,
    schema: SchemaRef,
}

impl PostgresTableProvider {
    /// Open a small pool to the datasource, fetch the table once (bounded by
    /// `max_fetch_rows`), and hold the materialised schema. Built eagerly so a
    /// planning-time schema error (missing table, bad creds) fails the query
    /// before execution. The table name is validated here so an invalid name
    /// never reaches SQL text.
    pub async fn connect(
        conn: &PostgresConn,
        table: &str,
        max_fetch_rows: usize,
    ) -> EngineResult<Self> {
        validate_identifier(table)?;
        let opts = PgConnectOptions::new()
            .host(&conn.host)
            .port(conn.port)
            .database(&conn.database)
            .username(&conn.user)
            .password(&conn.password);
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(opts)
            .await
            .map_err(|e| EngineError::Source(format!("federation connect failed: {e}")))?;
        // Fetch once at build time so a planning-time schema error (missing table,
        // bad creds) fails the query before execution; the schema is held and
        // `scan` re-fetches current rows.
        let batch = fetch(&pool, table, max_fetch_rows).await?;
        Ok(Self {
            pool,
            table: table.to_string(),
            max_fetch_rows,
            schema: batch.schema(),
        })
    }
}

/// Pull up to `limit` rows of `table` as JSON objects and build one Arrow batch.
async fn fetch(
    pool: &PgPool,
    table: &str,
    limit: usize,
) -> EngineResult<datafusion::arrow::array::RecordBatch> {
    // `to_jsonb(t)` yields one JSON object per row; the LIMIT bounds the pull so a
    // huge remote table cannot exhaust memory before the output caps apply.
    let sql = format!("SELECT to_jsonb(t) AS row FROM \"{table}\" t LIMIT {limit}");
    let pg_rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| EngineError::Source(format!("federation fetch failed: {e}")))?;
    let rows: Vec<Value> = pg_rows
        .iter()
        .map(|r| r.try_get::<Value, _>("row"))
        .collect::<Result<_, _>>()
        .map_err(|e| EngineError::Source(format!("federation row decode failed: {e}")))?;
    json_rows_to_batch(&rows)
}

#[async_trait]
impl TableProvider for PostgresTableProvider {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }

    fn table_type(&self) -> TableType {
        TableType::Base
    }

    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> DfResult<Arc<dyn ExecutionPlan>> {
        // Re-fetch per scan so a federated query always sees current rows. The
        // bound is the smaller of the planner's limit (if any) and the
        // provider's configured cap, so projection/filter/limit then apply above
        // the in-memory table.
        let bound = limit.map_or(self.max_fetch_rows, |l| l.min(self.max_fetch_rows));
        let batch = fetch(&self.pool, &self.table, bound)
            .await
            .map_err(|e| DataFusionError::External(Box::new(e)))?;
        let mem = MemTable::try_new(batch.schema(), vec![vec![batch]])?;
        mem.scan(state, projection, filters, limit).await
    }
}
