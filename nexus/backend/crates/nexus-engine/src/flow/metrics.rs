//! Per-flow ingest counters shared between a running pipeline and the flows API.
//!
//! The flow manager hands one [`FlowMetrics`] handle into the metered source/sink
//! wrappers it builds for a run; those wrappers bump the atomics as batches flow,
//! and the API reads a [`MetricsSnapshot`] for the flows list/detail without
//! locking the pipeline. The counters are process-local and reset when a fresh
//! run starts (the manager installs a new handle per `start`), so they describe
//! the current run, not the lifetime of the flow.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;

/// Atomic ingest counters for one flow run. Cheap to clone — every field lives
/// behind one shared `Arc`, so the source wrapper, the sink wrapper, and the API
/// reader all observe the same counts.
#[derive(Clone, Default)]
pub struct FlowMetrics {
    inner: Arc<Counters>,
}

/// The raw atomics. `in_flight` is a live gauge (batches read but not yet
/// written) the snapshot reports as the channel depth; everything else is a
/// monotonic counter. `last_write_ms` is the most recent successful sink-write
/// wall time in milliseconds.
#[derive(Default)]
struct Counters {
    batches_in: AtomicU64,
    rows_written: AtomicU64,
    in_flight: AtomicI64,
    flush_count: AtomicU64,
    write_errors: AtomicU64,
    last_write_ms: AtomicU64,
}

impl FlowMetrics {
    /// A fresh zeroed handle for one run.
    pub fn new() -> Self {
        Self::default()
    }

    /// Count one batch entering the pipeline from the source and raise the
    /// in-flight gauge. The sink lowers the gauge on write, so it tracks the depth
    /// of the bounded source→sink channel without reaching into the pipeline.
    pub fn record_batch_in(&self) {
        self.inner.batches_in.fetch_add(1, Ordering::Relaxed);
        self.inner.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Count `rows` successfully written by the sink, stamp the write time, and
    /// lower the in-flight gauge by one batch.
    pub fn record_write(&self, rows: usize, at_ms: u64) {
        self.inner
            .rows_written
            .fetch_add(rows as u64, Ordering::Relaxed);
        self.inner.last_write_ms.store(at_ms, Ordering::Relaxed);
        self.inner.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Count one sink flush (a batch handed to the underlying writer).
    pub fn record_flush(&self) {
        self.inner.flush_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Count one failed sink write attempt (each retry attempt that errored).
    pub fn record_write_error(&self) {
        self.inner.write_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Read a consistent-enough point-in-time view for the API. Relaxed loads are
    /// fine: the flows endpoint reports approximate live counts, not a ledger.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let c = &self.inner;
        MetricsSnapshot {
            batches_in: c.batches_in.load(Ordering::Relaxed),
            rows_written: c.rows_written.load(Ordering::Relaxed),
            channel_depth: c.in_flight.load(Ordering::Relaxed).max(0) as u64,
            flush_count: c.flush_count.load(Ordering::Relaxed),
            write_errors: c.write_errors.load(Ordering::Relaxed),
            last_write_ms: match c.last_write_ms.load(Ordering::Relaxed) {
                0 => None,
                ms => Some(ms),
            },
        }
    }
}

/// A point-in-time copy of a run's counters for the flows API to serialise.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MetricsSnapshot {
    /// Batches read from the source since this run started.
    pub batches_in: u64,
    /// Rows the sink has successfully written since this run started.
    pub rows_written: u64,
    /// Approximate depth of the source→sink bounded channel: batches read but
    /// not yet drained to the sink. A coarse backpressure gauge — it can briefly
    /// undershoot under processor fan-out and is clamped at zero.
    pub channel_depth: u64,
    /// Number of batches handed to the underlying sink writer.
    pub flush_count: u64,
    /// Failed sink write attempts (each errored attempt, including retries).
    pub write_errors: u64,
    /// Wall-clock millis of the last successful write, or `None` if none yet.
    pub last_write_ms: Option<u64>,
}
