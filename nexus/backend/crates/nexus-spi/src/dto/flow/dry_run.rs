//! `POST /api/v1/flows/dry-run` request/response — validate + bounded test run.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

use crate::dto::query::{ColumnSchema, QueryStats};

/// Test a flow's `input` + `pipeline` without persisting it or running its real
/// output. The engine swaps in the bounded collector sink, runs the stream to
/// the first breached cap (or a short wall-clock deadline), and returns the
/// sample rows. `output` is intentionally absent — a dry run never writes to a
/// real sink. Optional `max_rows` lets the editor narrow the sample further
/// than the server default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DryRunRequest {
    /// The flow's input config blob (`{type, ...}`).
    pub input: Value,
    /// The pipeline processors, in order. Defaults to an empty pipeline.
    #[serde(default)]
    pub pipeline: Option<Value>,
    /// Cap on sample rows; clamped to the server's hard maximum.
    #[serde(default)]
    pub max_rows: Option<u64>,
}

/// A dry run's outcome. On a build/runtime failure `error` carries the message
/// and `rows` is empty; otherwise `rows`/`columns`/`stats` describe the bounded
/// sample (with `stats.truncated` set when a cap stopped the stream early).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct DryRunResponse {
    /// Column schema of the sample, derived from the result's Arrow schema.
    pub columns: Vec<ColumnSchema>,
    /// The sample rows, JSON-encoded.
    pub rows: Vec<Value>,
    /// Row/byte/time counters and the truncation flag for the sample.
    pub stats: QueryStats,
    /// A build or runtime error, surfaced before save. `None` on success.
    pub error: Option<String>,
}
