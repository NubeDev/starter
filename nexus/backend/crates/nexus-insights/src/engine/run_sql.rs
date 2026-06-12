//! Run one SQL statement over a frame's batches and collect the result.
//!
//! Every primitive lowers to a single SQL statement against a one-shot table
//! named [`TABLE`]. DataFusion's planning + execution are async; an insight runs
//! inside `spawn_blocking`, so a current Tokio runtime handle is always present
//! and we `block_on` it here. This keeps the Rhai-facing primitive methods
//! synchronous (Rhai has no async) without spawning a nested runtime.

use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::datasource::MemTable;
use datafusion::prelude::SessionContext;

use crate::error::{InsightError, InsightResult};

/// The fixed table name each primitive's SQL reads from. Scripts never see it —
/// it exists only between a primitive's input frame and its lowered statement.
pub const TABLE: &str = "frame";

/// Register `batches` (with `schema`) as [`TABLE`] in a fresh context, run `sql`,
/// and collect the result batches. A fresh context per call keeps every primitive
/// independent and stateless — there is no cross-call catalog to leak between
/// tenants or executions.
pub fn run(
    schema: SchemaRef,
    batches: Vec<RecordBatch>,
    sql: &str,
) -> InsightResult<Vec<RecordBatch>> {
    let ctx = SessionContext::new();
    let partitions = vec![batches];
    let table = MemTable::try_new(schema, partitions)
        .map_err(|e| InsightError::Engine(format!("register frame: {e}")))?;
    ctx.register_table(TABLE, Arc::new(table))
        .map_err(|e| InsightError::Engine(format!("register table: {e}")))?;

    let handle = tokio::runtime::Handle::current();
    handle.block_on(async {
        let df = ctx
            .sql(sql)
            .await
            .map_err(|e| InsightError::Runtime(format!("{e}")))?;
        df.collect()
            .await
            .map_err(|e| InsightError::Runtime(format!("{e}")))
    })
}
