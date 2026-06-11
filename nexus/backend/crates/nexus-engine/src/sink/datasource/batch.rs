//! The write-side batching policy: accumulate rows until a row-count or a time
//! threshold, then hand one combined batch to the writer.
//!
//! A device flow emits many small batches; writing each one straight through
//! defeats the bulk-write primitives (COPY, Parquet row groups). This accumulator
//! buffers incoming batches and reports when the buffer is due to flush — on
//! `batch_rows` reached OR `batch_ms` elapsed since the first buffered batch,
//! whichever comes first. The sink owns the writer and the wall clock; this type
//! owns only the decision and the buffer, which keeps it unit-testable under
//! paused tokio time with no I/O. This is the write half of the roadmap §6
//! backpressure contract that RW-08 soak-tests.

use std::time::Duration;

use datafusion::arrow::array::RecordBatch;
use datafusion::arrow::compute::concat_batches;
use tokio::time::Instant;

use crate::core::{EngineError, EngineResult};

/// Buffers batches and decides when they are due to flush.
pub struct BatchAccumulator {
    rows_threshold: usize,
    time_threshold: Duration,
    buffered: Vec<RecordBatch>,
    rows: usize,
    /// When the first still-buffered batch arrived, for the time threshold.
    /// `None` while the buffer is empty.
    first_at: Option<Instant>,
}

impl BatchAccumulator {
    /// Build with a row count and a time window. Both must be positive; a zero
    /// row threshold would flush every batch (defeating batching) and a zero time
    /// window would never accumulate, so each is floored to at least one unit.
    pub fn new(rows_threshold: usize, time_threshold: Duration) -> Self {
        Self {
            rows_threshold: rows_threshold.max(1),
            time_threshold: time_threshold.max(Duration::from_millis(1)),
            buffered: Vec::new(),
            rows: 0,
            first_at: None,
        }
    }

    /// Add a batch to the buffer. Empty batches are ignored so they neither start
    /// the timer nor count toward the row threshold.
    pub fn push(&mut self, batch: RecordBatch) {
        if batch.num_rows() == 0 {
            return;
        }
        if self.first_at.is_none() {
            self.first_at = Some(Instant::now());
        }
        self.rows += batch.num_rows();
        self.buffered.push(batch);
    }

    /// Whether the buffer has reached the row threshold. Checked after each
    /// `push` for the immediate (count-driven) flush path.
    pub fn rows_due(&self) -> bool {
        self.rows >= self.rows_threshold
    }

    /// Whether the time window has elapsed since the first buffered batch. The
    /// sink consults this on its timer tick for the time-driven flush path.
    pub fn time_due(&self) -> bool {
        match self.first_at {
            Some(t) => t.elapsed() >= self.time_threshold,
            None => false,
        }
    }

    /// Drain the buffer into one combined batch, resetting the accumulator.
    /// Returns `None` when nothing is buffered. Concatenation is the cost of the
    /// bulk-write payoff; the schema-stability contract guarantees the buffered
    /// batches share a schema, so `concat_batches` cannot fail on a mismatch.
    pub fn drain(&mut self) -> EngineResult<Option<RecordBatch>> {
        if self.buffered.is_empty() {
            return Ok(None);
        }
        let schema = self.buffered[0].schema();
        let combined = concat_batches(&schema, self.buffered.iter())
            .map_err(|e| EngineError::Sink(format!("batch concat failed: {e}")))?;
        self.buffered.clear();
        self.rows = 0;
        self.first_at = None;
        Ok(Some(combined))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use datafusion::arrow::array::Int64Array;
    use datafusion::arrow::datatypes::{DataType, Field, Schema};

    /// A one-column Int64 batch with `n` rows, to drive the row/time thresholds.
    fn batch(n: usize) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Int64, false)]));
        let col = Arc::new(Int64Array::from_iter_values(0..n as i64));
        RecordBatch::try_new(schema, vec![col]).unwrap()
    }

    #[test]
    fn flushes_when_the_row_threshold_is_reached() {
        let mut acc = BatchAccumulator::new(3, Duration::from_secs(60));
        acc.push(batch(2));
        assert!(!acc.rows_due(), "two of three rows is not yet due");
        acc.push(batch(1));
        assert!(acc.rows_due(), "the third row trips the count threshold");
        let drained = acc.drain().unwrap().unwrap();
        assert_eq!(drained.num_rows(), 3);
        assert!(acc.drain().unwrap().is_none(), "drain resets the buffer");
    }

    #[tokio::test(start_paused = true)]
    async fn flushes_on_the_time_window_independent_of_row_count() {
        // A large row threshold the buffer never reaches, so only the timer can fire.
        let mut acc = BatchAccumulator::new(1_000_000, Duration::from_millis(500));
        acc.push(batch(1));
        assert!(!acc.time_due(), "the window has not elapsed yet");
        tokio::time::advance(Duration::from_millis(600)).await;
        assert!(acc.time_due(), "past the window, a sub-threshold buffer is due");
        assert!(!acc.rows_due(), "the row threshold was never reached");
    }

    #[test]
    fn empty_batches_neither_buffer_nor_arm_the_timer() {
        let mut acc = BatchAccumulator::new(1, Duration::from_millis(1));
        acc.push(batch(0));
        assert!(!acc.rows_due());
        assert!(!acc.time_due(), "an empty push must not start the timer");
        assert!(acc.drain().unwrap().is_none());
    }
}
