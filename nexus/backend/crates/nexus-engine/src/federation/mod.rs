//! Federated query: run one read-only SQL statement across several resolved
//! datasources and/or local files via DataFusion.
//!
//! This is the alternate query path the API dispatch seam picks when a request
//! references more than one datasource or a file kind; a single-datasource
//! request stays on the existing push-down path untouched. The caller resolves
//! each referenced datasource into a [`FederatedSource`] (decrypting creds
//! through the audited envelope path) and supplies the SQL plus the same caps the
//! one-shot `QueryRunner` enforces. Execution is bounded on both sides: a
//! `MemoryPool` caps working memory ([`context`]) and the [`Caps`] cap the
//! output (rows/bytes/wall-clock), with truncation surfaced exactly as the
//! collector does.

mod context;
mod identifier;
mod postgres_table;
mod rows_to_batch;
mod source;

use std::time::Instant;

use datafusion::prelude::SQLOptions;
use nexus_spi::dto::query::QueryStats;
use tokio_util::sync::CancellationToken;

use crate::arrow_json::{batch_to_rows, columns_of};
use crate::core::{EngineError, EngineResult};
use crate::runner::cancel;
use crate::runner::QueryOutcome;
use crate::sink::cap::{CapState, Caps};

pub use source::{FederatedSource, PostgresConn};

/// How many rows each remote SQL source may pull per scan before the join sees
/// them. The output caps bound the result, but a join can buffer two large
/// inputs first; this bounds the inputs. Conservative by default — federation is
/// the exception path, not the hot path.
const DEFAULT_MAX_FETCH_ROWS: usize = 100_000;

/// Working-memory budget for the DataFusion runtime, in bytes. A join exceeding
/// it fails with a resource error rather than an OOM. 256 MiB leaves headroom on
/// a control-plane node without starving a legitimate cross-source join.
const DEFAULT_MAX_INPUT_BYTES: usize = 256 * 1024 * 1024;

/// A resolved federated query: the SQL plus its authorised `(alias, source)`
/// inputs. The alias is the SQL-visible table (`ds_<alias>`); the API layer fills
/// `sources` only for datasources the caller's tenant owns, so an alias here is
/// already authz-cleared.
pub struct FederatedQuery {
    /// The read-only SQL referencing `ds_<alias>` tables.
    pub sql: String,
    /// Resolved inputs, one per alias. Order is irrelevant to planning.
    pub sources: Vec<(String, FederatedSource)>,
}

impl FederatedQuery {
    /// Run the query under `caps` and return the collected rows + stats. A breached
    /// row/byte cap truncates the result (reported via `stats.truncated`) rather
    /// than erroring; the wall-clock cap cancels the run. DDL/DML/statements are
    /// rejected so a federated query is a pure read, like the `sql` processor.
    pub async fn run(&self, caps: Caps) -> EngineResult<QueryOutcome> {
        let started = Instant::now();
        let token = CancellationToken::new();
        let timer = cancel::deadline(token.clone(), caps.max_duration);

        let result = self.execute(caps, token).await;

        if let Some(timer) = timer {
            timer.abort();
        }
        let elapsed_ms = started.elapsed().as_millis() as u64;
        finish(result, elapsed_ms)
    }

    /// Build the context, plan + run the SQL, and collect batches under the caps.
    /// Returns the partial collection alongside whether it was truncated so the
    /// caller can report a capped result without treating it as a failure.
    async fn execute(&self, caps: Caps, token: CancellationToken) -> EngineResult<Collected> {
        let ctx = context::build(
            &self.sources,
            DEFAULT_MAX_INPUT_BYTES,
            DEFAULT_MAX_FETCH_ROWS,
        )
        .await?;

        let options = SQLOptions::new()
            .with_allow_ddl(false)
            .with_allow_dml(false)
            .with_allow_statements(false);
        let df = ctx
            .sql_with_options(&self.sql, options)
            .await
            .map_err(|e| EngineError::Processor(format!("federation plan: {e}")))?;
        let batches = df
            .collect()
            .await
            .map_err(|e| EngineError::Processor(format!("federation execute: {e}")))?;

        collect(batches, caps, &token)
    }
}

/// The rows collected from a run plus the cap-tracking state.
struct Collected {
    columns: Vec<nexus_spi::dto::query::ColumnSchema>,
    rows: Vec<serde_json::Value>,
    state: CapState,
}

/// Convert each result batch to JSON rows, admitting them against the caps and
/// stopping at the first batch that would breach a limit (the result is then
/// truncated, mirroring the collector sink). The wall-clock token is checked so a
/// late timeout stops the drain.
fn collect(
    batches: Vec<datafusion::arrow::array::RecordBatch>,
    caps: Caps,
    token: &CancellationToken,
) -> EngineResult<Collected> {
    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut state = CapState::default();
    for batch in batches {
        if token.is_cancelled() {
            state.truncated = true;
            break;
        }
        if batch.num_rows() == 0 {
            continue;
        }
        if columns.is_empty() {
            columns = columns_of(&batch);
        }
        let json = batch_to_rows(&batch).map_err(EngineError::Processor)?;
        let n = json.rows.len() as u64;
        if !state.admit(n, json.bytes, &caps) {
            break;
        }
        rows.extend(json.rows);
    }
    Ok(Collected {
        columns,
        rows,
        state,
    })
}

/// Shape the executed collection into a `QueryOutcome`, or propagate a hard
/// engine error when nothing was collected and nothing was truncated. A truncated
/// result stands — a breached cap is an expected stop, not a failure.
fn finish(result: EngineResult<Collected>, elapsed_ms: u64) -> EngineResult<QueryOutcome> {
    let collected = result?;
    let row_count = collected.rows.len() as u64;
    Ok(QueryOutcome {
        columns: collected.columns,
        rows: collected.rows,
        stats: QueryStats {
            row_count,
            byte_count: collected.state.bytes,
            elapsed_ms,
            truncated: collected.state.truncated,
        },
    })
}
