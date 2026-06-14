//! `Frame` — the script-visible handle over the current Arrow batches.
//!
//! A frame is immutable: every primitive returns a *new* frame, so a script
//! chains `df.resample(...).zscore("kw").anomalies("kw", 3.0)` without aliasing.
//! The methods here are the engine-side implementation; `api.rs` wraps them as
//! the Rhai custom-type surface. Column names are validated and quoted before
//! they reach SQL so a column name can never inject — the only free-form values
//! a primitive accepts are column identifiers (checked against the live schema)
//! and numeric literals (formatted by us, never the script's text).

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::datatypes::SchemaRef;

use super::convert::empty_schema;
use super::run_sql;
use crate::error::{InsightError, InsightResult};

/// A handle over a set of Arrow batches sharing one schema. Clone is cheap —
/// batches are `Arc`-backed columns.
#[derive(Clone)]
pub struct Frame {
    schema: SchemaRef,
    batches: Vec<RecordBatch>,
}

impl Frame {
    /// Build a frame from raw batches. An empty input is an empty schema frame.
    pub fn from_batches(batches: Vec<RecordBatch>) -> Self {
        let schema = batches
            .first()
            .map(RecordBatch::schema)
            .unwrap_or_else(empty_schema);
        Self { schema, batches }
    }

    /// The frame's batches, for serialization back to rows.
    pub fn batches(&self) -> &[RecordBatch] {
        &self.batches
    }

    /// Total row count across all batches.
    pub fn row_count(&self) -> usize {
        self.batches.iter().map(RecordBatch::num_rows).sum()
    }

    /// The frame's column names, in schema order.
    pub fn columns(&self) -> Vec<String> {
        self.schema
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect()
    }

    /// Reject a column name that is not in the schema, naming the offender so the
    /// tenant sees a clear runtime error rather than a planner failure.
    pub(super) fn require_column(&self, col: &str) -> InsightResult<()> {
        if self.schema.field_with_name(col).is_err() {
            return Err(InsightError::Runtime(format!("no such column: {col}")));
        }
        Ok(())
    }

    /// Run one lowered SQL statement over this frame and wrap the result. Used by
    /// every primitive in `ops`.
    pub(super) fn query(&self, sql: &str) -> InsightResult<Frame> {
        let batches = run_sql::run(self.schema.clone(), self.batches.clone(), sql)?;
        Ok(Frame::from_batches(batches))
    }
}
