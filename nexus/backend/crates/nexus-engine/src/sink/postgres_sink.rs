//! The native `postgres` sink: insert each batch's rows into a table in a
//! datasource Postgres.
//!
//! On the [`Sink`] trait: each row is converted to a JSON object via the shared
//! Arrow→JSON bridge and inserted with bound parameters (see
//! [`super::pg_insert`]), never string-concatenated. The pool is opened lazily on
//! the first write and closed on `close`. The connection string comes from the
//! flow config (the datasource secret, decrypted by the caller at build time),
//! never from a request.

use serde::Deserialize;
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

use datafusion::arrow::array::RecordBatch;

use super::pg_insert::insert_row;
use crate::arrow_json::batch_to_rows;
use crate::core::{EngineError, EngineResult, Sink};

#[derive(Debug, Clone, Deserialize)]
struct PostgresConfig {
    /// Connection string for the target Postgres.
    uri: String,
    /// Table the shaped rows are inserted into.
    table: String,
}

/// Inserts batch rows into a datasource Postgres table, opening its pool on the
/// first write.
pub struct PostgresSink {
    uri: String,
    table: String,
    pool: Option<PgPool>,
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
            pool: None,
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
}

#[async_trait::async_trait]
impl Sink for PostgresSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let rows = batch_to_rows(batch).map_err(EngineError::Sink)?.rows;
        // Split the borrow: take the table name out before borrowing the pool
        // mutably, so both live across the await.
        let table = self.table.clone();
        let pool = self.pool().await?.clone();
        for row in &rows {
            let obj = row
                .as_object()
                .ok_or_else(|| EngineError::Sink("postgres sink expects object rows".into()))?;
            insert_row(&pool, &table, obj).await?;
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
