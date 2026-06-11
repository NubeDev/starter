//! The three pipeline node traits — `Source`, `Processor`, `Sink` — over Arrow
//! `RecordBatch`.
//!
//! These signatures are the frozen engine contract (roadmap §6): RW-02 ports the
//! real nodes onto them, so they must not drift once that lane starts. All node
//! configuration enters as `serde_json::Value` through the registry builders, so
//! a saved flow config stays code-free; the traits themselves carry no config.

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;

use super::error::EngineResult;

/// A pipeline input. `read` yields the next batch, or `None` once the source is
/// exhausted — a finite source (a bounded query) returns `None` to end the run;
/// an infinite source (a live subscription) never does and is stopped only by
/// cancellation. Implementors should make `read` cancel-safe at an `.await`
/// point so the pipeline can drop a pending read without losing committed state.
#[async_trait]
pub trait Source: Send {
    /// Read the next batch. `Ok(None)` signals clean end-of-stream; `Ok(Some)`
    /// hands the next batch downstream; `Err` aborts the run.
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>>;

    /// Acknowledge that the batches read so far have been written by the sink.
    ///
    /// The "no silent data loss" contract (roadmap §6 delivery semantics): a
    /// QoS-capable source (MQTT, a queue) MUST NOT commit upstream delivery until
    /// the batch has actually landed in the sink, so the pipeline calls `commit`
    /// after each successful sink write. The default is a no-op — pull-only
    /// sources (memory, generate, HTTP poll, simulator) have no upstream offset to
    /// advance and are documented at-most-once for an in-flight batch; only a
    /// source that can defer an ack overrides this.
    async fn commit(&mut self) -> EngineResult<()> {
        Ok(())
    }
}

/// A batch transform applied sequentially in the pipeline. One input batch may
/// fan out to zero or more output batches (a filter that drops everything yields
/// an empty `Vec`; a split yields several). Takes `&mut self` so a stateful
/// transform — windowing, dedupe, schema-locking — keeps its state inline without
/// interior mutability; the pipeline owns the chain on one task and applies each
/// processor in order, so exclusive access costs nothing.
#[async_trait]
pub trait Processor: Send {
    /// Transform one batch into zero or more batches. `Err` aborts the run.
    async fn process(&mut self, batch: RecordBatch) -> EngineResult<Vec<RecordBatch>>;
}

/// A pipeline output. `write` is called once per batch in arrival order; `close`
/// is called exactly once after the last batch (on clean end or on cancellation
/// after the in-flight batch drains) to flush and release resources.
#[async_trait]
pub trait Sink: Send {
    /// Write one batch. `Err` aborts the run before `close` is reached.
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()>;

    /// Flush and release. Called exactly once at end-of-run, including after a
    /// cancellation, so a sink can rely on it for final commits.
    async fn close(&mut self) -> EngineResult<()>;
}
