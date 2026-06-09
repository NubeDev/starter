//! A custom ArkFlow output (`type: postgres`) that inserts each batch's rows
//! into a table in a datasource Postgres.
//!
//! The write end of light ingestion: a flow's pipeline shapes rows, and this
//! sink lands them in `table`. Each row is converted to a JSON object (reusing
//! the same Arrow→JSON path the query/SSE sinks use) and inserted with bound
//! parameters, so column values are never string-concatenated into SQL. The
//! connection string comes from the flow config (the datasource secret,
//! decrypted at build time by the caller), never from the request.

use std::sync::Arc;

use arkflow_core::codec::Codec;
use arkflow_core::output::{register_output_builder, Output, OutputBuilder};
use arkflow_core::{Error, MessageBatchRef, Resource};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Map, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, QueryBuilder};
use tokio::sync::Mutex;

use crate::arrow_json;

#[derive(Debug, Clone, Deserialize)]
struct PostgresConfig {
    /// Connection string for the target Postgres, e.g.
    /// `postgres://user:pass@host:5432/db`.
    uri: String,
    /// Table the shaped rows are inserted into.
    table: String,
}

struct PostgresOutput {
    uri: String,
    table: String,
    // A flow has one long-lived sink; the pool is opened on connect and shared
    // across writes. Behind a mutex only because the Output trait hands `&self`
    // and the pool is set after construction.
    pool: Mutex<Option<PgPool>>,
}

#[async_trait]
impl Output for PostgresOutput {
    async fn connect(&self) -> Result<(), Error> {
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .connect(&self.uri)
            .await
            .map_err(|e| Error::Config(format!("postgres output connect failed: {e}")))?;
        *self.pool.lock().await = Some(pool);
        Ok(())
    }

    async fn write(&self, msg: MessageBatchRef) -> Result<(), Error> {
        let batch = &**msg;
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let rows = arrow_json::batch_to_rows(batch).map_err(Error::Process)?.rows;

        let guard = self.pool.lock().await;
        let pool = guard
            .as_ref()
            .ok_or_else(|| Error::Process("postgres output not connected".into()))?;

        for row in &rows {
            let obj = row
                .as_object()
                .ok_or_else(|| Error::Process("postgres output expects object rows".into()))?;
            insert_row(pool, &self.table, obj).await?;
        }
        Ok(())
    }

    async fn close(&self) -> Result<(), Error> {
        if let Some(pool) = self.pool.lock().await.take() {
            pool.close().await;
        }
        Ok(())
    }
}

/// Insert one JSON object as a row. Columns are the object's keys (quoted, so a
/// reserved word is safe); values are bound, never interpolated. A non-scalar
/// value is bound as its JSON text — a deliberate, lossless fallback for nested
/// shapes rather than a silent drop.
async fn insert_row(pool: &PgPool, table: &str, obj: &Map<String, Value>) -> Result<(), Error> {
    let cols: Vec<&String> = obj.keys().collect();
    let column_list = cols
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let mut qb = QueryBuilder::<sqlx::Postgres>::new(format!("INSERT INTO {table} ({column_list}) "));
    qb.push_values(std::iter::once(()), |mut b, _| {
        for c in &cols {
            bind_value(&mut b, &obj[*c]);
        }
    });
    qb.build()
        .execute(pool)
        .await
        .map_err(|e| Error::Process(format!("postgres output insert failed: {e}")))?;
    Ok(())
}

fn bind_value<'a>(
    b: &mut sqlx::query_builder::Separated<'_, 'a, sqlx::Postgres, &'static str>,
    value: &'a Value,
) {
    match value {
        Value::Null => {
            b.push_bind(None::<String>);
        }
        Value::Bool(v) => {
            b.push_bind(*v);
        }
        Value::Number(n) if n.is_i64() => {
            b.push_bind(n.as_i64().unwrap());
        }
        Value::Number(n) if n.is_u64() => {
            b.push_bind(n.as_u64().unwrap() as i64);
        }
        Value::Number(n) => {
            b.push_bind(n.as_f64().unwrap_or(0.0));
        }
        Value::String(s) => {
            b.push_bind(s.clone());
        }
        // Arrays/objects round-trip as their JSON text — lossless, and the target
        // column can be text or jsonb.
        other => {
            b.push_bind(other.to_string());
        }
    }
}

struct PostgresOutputBuilder;

impl OutputBuilder for PostgresOutputBuilder {
    fn build(
        &self,
        _name: Option<&String>,
        config: &Option<Value>,
        _codec: Option<Arc<dyn Codec>>,
        _resource: &Resource,
    ) -> Result<Arc<dyn Output>, Error> {
        let config: PostgresConfig = config
            .clone()
            .ok_or_else(|| Error::Config("postgres output requires uri and table".into()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| Error::Config(format!("invalid postgres config: {e}")))
            })?;
        Ok(Arc::new(PostgresOutput {
            uri: config.uri,
            table: config.table,
            pool: Mutex::new(None),
        }))
    }
}

/// Register the `postgres` output type. Called once at startup.
pub fn init() -> Result<(), Error> {
    register_output_builder("postgres", Arc::new(PostgresOutputBuilder))
}
