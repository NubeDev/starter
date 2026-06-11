//! The native `drop` sink: discard every batch.
//!
//! The null output, for flows whose effect is entirely in their processors and
//! for tests that only need the source/processor path to run. Writing is a
//! no-op; there is nothing to flush on close.

use datafusion::arrow::array::RecordBatch;

use crate::core::{EngineResult, Sink};

/// Discards all batches.
#[derive(Default)]
pub struct DropSink;

impl DropSink {
    /// A new drop sink. The node config carries no fields.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Sink for DropSink {
    async fn write(&mut self, _batch: &RecordBatch) -> EngineResult<()> {
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        Ok(())
    }
}
