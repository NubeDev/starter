//! The native `stdout` sink: print each batch's rows as JSON to stdout.
//!
//! A debugging output for dry runs and local flow development — it writes one
//! JSON object per row to stdout via the shared Arrow→JSON bridge, so what a
//! flow would land in a datasource is visible without one. Nothing buffers, so
//! close has nothing to flush.

use datafusion::arrow::array::RecordBatch;

use crate::arrow_json::batch_to_rows;
use crate::core::{EngineError, EngineResult, Sink};

/// Prints each batch's rows as JSON to stdout.
#[derive(Default)]
pub struct StdoutSink;

impl StdoutSink {
    /// A new stdout sink. The node config carries no fields.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Sink for StdoutSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let converted = batch_to_rows(batch).map_err(EngineError::Sink)?;
        for row in converted.rows {
            println!("{row}");
        }
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        Ok(())
    }
}
