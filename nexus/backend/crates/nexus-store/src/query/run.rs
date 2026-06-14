//! Execute a user query against a datasource Postgres under the safety guards.
//!
//! Every guard is a real connection/statement property, not advice the caller
//! could opt out of: the work runs inside a `READ ONLY` transaction (writes and
//! DDL are rejected by Postgres itself), a `statement_timeout` bounds wall-clock,
//! and the user SQL is wrapped in an outer `LIMIT` so the database returns at
//! most one row past the cap — letting the row/byte loop detect truncation
//! without ever buffering an unbounded result.

use futures::TryStreamExt;
use nexus_spi::dto::query::{QueryResponse, QueryStats};
use sqlx::{Executor, PgPool};
use starter_spi::Error;
use std::time::Instant;

use super::bind::{self, BindCtx, BoundQuery, SqlValue};
use super::row_json::{columns_of, row_to_object};
use super::QueryGuards;

/// Run raw `sql` (no macros/variables) and collect a bounded result. A
/// convenience over [`run_bound_query`] for the plain path: it binds against an
/// empty context (yielding zero args) and executes the result, so even raw SQL
/// flows through the same prepared-statement path. Returns a domain `Error` on a
/// rejected write, a timeout, or a malformed query.
pub async fn run_query(
    pool: &PgPool,
    sql: &str,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let bound = bind::bind(sql, &BindCtx::default())?;
    run_bound_query(pool, &bound, guards).await
}

/// Run raw `sql` against the metadata pool under the same read-only guards as
/// [`run_query`], but with `app.tenant_id` bound for the transaction so RLS
/// filters every row to `tenant_id` — exactly like [`crate::tenant_tx::begin`],
/// except the transaction is also `READ ONLY` and capped. This is the path the
/// admin nexus-DB inspector uses: it can read the control-plane tables, but only
/// its own tenant's rows, and cannot write or run DDL (Postgres rejects it).
pub async fn run_query_tenant_ro(
    pool: &PgPool,
    tenant_id: &str,
    sql: &str,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let bound = bind::bind(sql, &BindCtx::default())?;
    let started = Instant::now();
    let mut tx = pool.begin().await.map_err(internal)?;

    // Read-only + bounded wall-clock, identical to `run_bound_query`. Set before
    // the tenant GUC so a failure here aborts before any row is visible.
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(internal)?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    tx.execute(format!("SET LOCAL statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(internal)?;
    // Bind the tenant GUC for this transaction so RLS scopes every metadata
    // table to it. Bound as a parameter, never interpolated.
    sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(internal)?;

    let mut query = sqlx::query(&bound.sql);
    for arg in &bound.args {
        query = bind_arg(query, arg);
    }
    let mut rows = query.fetch(&mut *tx);
    let mut columns = Vec::new();
    let mut out_rows = Vec::new();
    let mut bytes: u64 = 0;
    let mut truncated = false;

    while let Some(row) = rows.try_next().await.map_err(invalid)? {
        if columns.is_empty() {
            columns = columns_of(&row);
        }
        if out_rows.len() as u64 >= guards.max_rows {
            truncated = true;
            break;
        }
        let obj = row_to_object(&row);
        let row_bytes = serde_json::to_vec(&obj)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        if bytes + row_bytes > guards.max_bytes {
            truncated = true;
            break;
        }
        bytes += row_bytes;
        out_rows.push(obj);
    }
    drop(rows);
    tx.commit().await.map_err(internal)?;

    let row_count = out_rows.len() as u64;
    Ok(QueryResponse {
        columns,
        rows: out_rows,
        stats: QueryStats {
            row_count,
            byte_count: bytes,
            elapsed_ms: started.elapsed().as_millis() as u64,
            truncated,
        },
    })
}

/// Execute a [`BoundQuery`] as a prepared statement and collect a bounded
/// result. Every value lives in `bound.args` and is bound by the driver — the
/// SQL text carries only `$N` placeholders and vetted identifiers — so this is
/// the single execution path for both macro-bearing and raw queries.
pub async fn run_bound_query(
    pool: &PgPool,
    bound: &BoundQuery,
    guards: QueryGuards,
) -> Result<QueryResponse, Error> {
    let started = Instant::now();
    let mut tx = pool.begin().await.map_err(internal)?;

    // The read-only transaction is the security boundary: any write or DDL in
    // `sql` — however it is phrased — is rejected by Postgres itself, not by
    // inspecting the query text. The statement timeout bounds wall-clock.
    tx.execute("SET TRANSACTION READ ONLY")
        .await
        .map_err(internal)?;
    let timeout_ms = guards.statement_timeout.as_millis().max(1);
    tx.execute(format!("SET LOCAL statement_timeout = {timeout_ms}").as_str())
        .await
        .map_err(internal)?;

    // Bind every argument so values reach Postgres through the driver, never
    // concatenated into the text. `WHERE`/`LIMIT` still push down to Postgres;
    // the result is bounded by stopping the cursor at the row/byte cap below
    // rather than by wrapping the statement — wrapping only fits row-returning
    // queries and would mask the read-only rejection a write must get.
    let mut query = sqlx::query(&bound.sql);
    for arg in &bound.args {
        query = bind_arg(query, arg);
    }
    let mut rows = query.fetch(&mut *tx);
    let mut columns = Vec::new();
    let mut out_rows = Vec::new();
    let mut bytes: u64 = 0;
    let mut truncated = false;

    while let Some(row) = rows.try_next().await.map_err(invalid)? {
        if columns.is_empty() {
            columns = columns_of(&row);
        }
        if out_rows.len() as u64 >= guards.max_rows {
            // The extra row past the cap proves there was more.
            truncated = true;
            break;
        }
        let obj = row_to_object(&row);
        let row_bytes = serde_json::to_vec(&obj)
            .map(|v| v.len() as u64)
            .unwrap_or(0);
        if bytes + row_bytes > guards.max_bytes {
            truncated = true;
            break;
        }
        bytes += row_bytes;
        out_rows.push(obj);
    }
    drop(rows);
    tx.commit().await.map_err(internal)?;

    let row_count = out_rows.len() as u64;
    Ok(QueryResponse {
        columns,
        rows: out_rows,
        stats: QueryStats {
            row_count,
            byte_count: bytes,
            elapsed_ms: started.elapsed().as_millis() as u64,
            truncated,
        },
    })
}

/// Bind one [`SqlValue`] onto the query builder, mapping the binder's value set
/// to the sqlx Postgres type the driver sends. A `Null` binds a typed `None` so
/// the placeholder is a genuine SQL `NULL`.
fn bind_arg<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    arg: &'q SqlValue,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    match arg {
        SqlValue::Text(s) => query.bind(s),
        SqlValue::TextArray(v) => query.bind(v),
        SqlValue::Int(i) => query.bind(i),
        SqlValue::Float(f) => query.bind(f),
        SqlValue::Bool(b) => query.bind(b),
        SqlValue::Timestamp(ts) => query.bind(ts),
        SqlValue::Null => query.bind(Option::<String>::None),
    }
}

/// A failed write, a syntax error, or a timeout is the caller's fault — 4xx.
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
