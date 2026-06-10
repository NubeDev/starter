//! The native `sql` processor: run a DataFusion SQL statement over the in-flight
//! batch.
//!
//! Each call registers the incoming batch under the configured table name
//! (`flow` by default — the name stored flow configs and the `SELECT … FROM
//! flow` query convention already use), executes the statement, and returns the
//! result. DDL, DML, and bare statements are rejected so a flow's SQL is a pure
//! read over its own batch, never a side-effecting command.

use std::sync::Arc;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::compute::concat_batches;
use datafusion::arrow::datatypes::Schema;
use datafusion::prelude::{SQLOptions, SessionContext};
use serde::Deserialize;
use serde_json::Value;

use crate::core::{EngineError, EngineResult, Processor};

/// The table name a batch is registered under when the config omits
/// `table_name`. Matches the `FROM flow` convention in stored flow configs.
const DEFAULT_TABLE_NAME: &str = "flow";

#[derive(Debug, Clone, Deserialize)]
struct SqlConfig {
    /// The SQL statement to run over the batch.
    query: String,
    /// Table name the batch is registered under (default [`DEFAULT_TABLE_NAME`]).
    #[serde(default)]
    table_name: Option<String>,
}

/// Runs a fixed read-only SQL statement over each batch via DataFusion.
pub struct SqlProcessor {
    query: String,
    table_name: String,
}

impl SqlProcessor {
    /// Build from the node config, requiring a `query`. The statement is not
    /// planned here — DataFusion plans it per batch against that batch's schema,
    /// since a flow's batches share one schema (enforced by `json_to_arrow`).
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: SqlConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid sql config: {e}")))?;
        Ok(Self {
            query: config.query,
            table_name: config
                .table_name
                .unwrap_or_else(|| DEFAULT_TABLE_NAME.to_string()),
        })
    }
}

#[async_trait::async_trait]
impl Processor for SqlProcessor {
    async fn process(&mut self, batch: RecordBatch) -> EngineResult<Vec<RecordBatch>> {
        if batch.num_rows() == 0 {
            return Ok(Vec::new());
        }
        let result = run_query(&self.query, &self.table_name, batch).await?;
        Ok(vec![result])
    }
}

/// Register `batch` as `table`, run `query`, and concatenate the result batches
/// into one (the collector and SSE sinks expect one batch per input batch). A
/// fresh `SessionContext` per call keeps the registration isolated — a flow runs
/// few enough batches per second that the setup cost is immaterial.
async fn run_query(query: &str, table: &str, batch: RecordBatch) -> EngineResult<RecordBatch> {
    let ctx = SessionContext::new();
    ctx.register_batch(table, batch)
        .map_err(|e| EngineError::Processor(format!("sql register batch: {e}")))?;

    // Read-only: no DDL/DML/statements, so a flow's SQL cannot mutate state.
    let options = SQLOptions::new()
        .with_allow_ddl(false)
        .with_allow_dml(false)
        .with_allow_statements(false);
    let df = ctx
        .sql_with_options(query, options)
        .await
        .map_err(|e| EngineError::Processor(format!("sql plan: {e}")))?;
    let batches = df
        .collect()
        .await
        .map_err(|e| EngineError::Processor(format!("sql execute: {e}")))?;

    match batches.as_slice() {
        [] => Ok(RecordBatch::new_empty(Arc::new(Schema::empty()))),
        [single] => Ok(single.clone()),
        many => concat_batches(&many[0].schema(), many)
            .map_err(|e| EngineError::Processor(format!("sql concat: {e}"))),
    }
}
