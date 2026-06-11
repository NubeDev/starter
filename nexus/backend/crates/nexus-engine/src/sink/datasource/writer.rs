//! The `DatasourceWriter` seam — one trait, one impl per datasource kind.
//!
//! The datasource sink batches rows kind-agnostically (see [`super::batch`]) and
//! hands a full batch to a writer chosen by the resolved datasource kind. Adding
//! a kind (an RW-07 extension, a future connector) is a new impl registered in
//! [`super::build`], with no change to the batching or the sink loop. Keeping the
//! writer behind a trait is what lets the postgres COPY path and the Parquet file
//! path share the sink's flush/close contract.

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;

use crate::core::EngineResult;

/// A kind-specific batch writer behind the datasource sink.
///
/// `write_batch` persists one accumulated batch (already bounded by the sink's
/// `batch_rows`/`batch_ms` policy); `flush` forces any writer-held buffer out and
/// is called on the sink's `close` (clean end or cancellation) so no row is left
/// unwritten. A writer that buffers nothing of its own keeps `flush` a no-op.
#[async_trait]
pub trait DatasourceWriter: Send {
    /// Persist one batch. `Err` aborts the run via [`crate::core::EngineError::Sink`].
    async fn write_batch(&mut self, batch: &RecordBatch) -> EngineResult<()>;

    /// Flush and finalize any writer-internal buffering. Called once at
    /// end-of-run, including after cancellation.
    async fn flush(&mut self) -> EngineResult<()>;
}
