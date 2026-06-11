//! A `Sink` wrapper that records write metrics and applies the flow's `on_error`
//! policy with capped retry/backoff.
//!
//! Wraps the real sink the registry built. Each write is attempted with capped
//! exponential backoff; every failed attempt is counted. Once retries are
//! exhausted the policy decides the terminal action (roadmap §6, RW-08 scope):
//! `halt` surfaces the error so the run stops and the flow records `last_error`
//! (the in-flight batch is never silently dropped); `drop` discards the batch,
//! counts it, and continues; `dlq` writes the batch to a dead-letter Parquet sink
//! and continues. `close` flushes the underlying sink and, for `dlq`, the
//! dead-letter writer too, so no rows are stranded.

use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;

use crate::core::{EngineError, EngineResult, Sink};
use crate::flow::metrics::FlowMetrics;
use crate::flow::policy::{SinkOnError, SinkPolicy};

/// Decorates a sink with per-flow write metrics and the `on_error` policy.
pub struct MeteredSink {
    inner: Box<dyn Sink>,
    metrics: FlowMetrics,
    policy: SinkPolicy,
    /// Lazily-opened dead-letter sink; only built when `on_error` is `dlq` and a
    /// write actually fails, so a healthy `dlq` flow opens no dead-letter file.
    dlq: Option<Box<dyn Sink>>,
    /// The flow id whose debug stream receives retry/drop/dlq log lines. `None`
    /// outside a managed flow run (e.g. tests), where logging is skipped.
    flow_id: Option<String>,
}

impl MeteredSink {
    /// Wrap `inner` with the flow's `metrics` handle and write-error `policy`.
    pub fn new(inner: Box<dyn Sink>, metrics: FlowMetrics, policy: SinkPolicy) -> Self {
        Self {
            inner,
            metrics,
            policy,
            dlq: None,
            flow_id: None,
        }
    }

    /// Bind the flow id so policy events publish to its debug log stream.
    pub fn with_flow_id(mut self, flow_id: impl Into<String>) -> Self {
        self.flow_id = Some(flow_id.into());
        self
    }

    /// Emit a debug log line to the flow's stream, if bound.
    fn debug_log(&self, level: nexus_spi::dto::flow::LogLevel, message: String) {
        if let Some(id) = &self.flow_id {
            crate::flow::debug::log(id, level, None, message);
        }
    }

    /// Attempt the write with capped retry/backoff. `Ok(())` on eventual success;
    /// `Err` once attempts are exhausted, for the caller to apply the terminal
    /// action. Every errored attempt is counted.
    async fn write_with_retry(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        let mut attempt = 0u32;
        loop {
            match self.inner.write(batch).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    self.metrics.record_write_error();
                    if attempt >= self.policy.backoff.max_attempts {
                        return Err(e);
                    }
                    attempt += 1;
                    tracing::warn!(
                        attempt,
                        error = %e,
                        "flow sink write failed; retrying after backoff"
                    );
                    self.debug_log(
                        nexus_spi::dto::flow::LogLevel::Warn,
                        format!("sink write failed (attempt {attempt}), retrying: {e}"),
                    );
                    tokio::time::sleep(self.policy.backoff.delay_for(attempt)).await;
                }
            }
        }
    }

    /// Route a batch that exhausted retries to the dead-letter sink, opening it on
    /// first use. A dlq-write failure is itself surfaced (halts the run) — a broken
    /// dead-letter path must not silently drop, which is the whole point of `dlq`.
    async fn to_dlq(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if self.dlq.is_none() {
            let cfg = self.policy.dlq.as_ref().ok_or_else(|| {
                EngineError::Sink("dlq policy selected but no dlq config present".into())
            })?;
            self.dlq = Some(Box::new(crate::sink::DatasourceSink::from_config(cfg)?));
        }
        let dlq = self.dlq.as_mut().expect("dlq opened above");
        dlq.write(batch).await
    }
}

#[async_trait]
impl Sink for MeteredSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        self.metrics.record_flush();
        match self.write_with_retry(batch).await {
            Ok(()) => {
                self.metrics.record_write(batch.num_rows(), now_ms());
                Ok(())
            }
            Err(e) => match self.policy.on_error {
                // Halt: surface the error so the pipeline stops and the flow
                // records `last_error`. The batch is not silently dropped.
                SinkOnError::Halt => Err(e),
                // Drop: discard the batch and keep running. The dropped rows are
                // counted via the write-error counter for each failed attempt.
                SinkOnError::Drop => {
                    tracing::warn!(error = %e, rows = batch.num_rows(), "flow sink dropping batch per on_error=drop");
                    self.debug_log(
                        nexus_spi::dto::flow::LogLevel::Warn,
                        format!("dropping {} rows per on_error=drop: {e}", batch.num_rows()),
                    );
                    Ok(())
                }
                // Dlq: persist the failed batch to the dead-letter writer.
                SinkOnError::Dlq => {
                    tracing::warn!(error = %e, rows = batch.num_rows(), "flow sink dead-lettering batch per on_error=dlq");
                    self.debug_log(
                        nexus_spi::dto::flow::LogLevel::Warn,
                        format!("dead-lettering {} rows per on_error=dlq: {e}", batch.num_rows()),
                    );
                    self.to_dlq(batch).await
                }
            },
        }
    }

    async fn close(&mut self) -> EngineResult<()> {
        let primary = self.inner.close().await;
        // Always flush the dead-letter sink too, so a partial final dlq buffer is
        // not stranded. The primary close error wins if both fail.
        if let Some(dlq) = self.dlq.as_mut() {
            let dlq_close = dlq.close().await;
            primary.and(dlq_close)
        } else {
            primary
        }
    }
}

/// Current wall-clock time in millis since the epoch, for the last-write stamp.
/// A clock before the epoch is impossible in practice; clamp to zero rather than
/// panic.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// A sink whose `write` fails `count` times before succeeding — the seam the
/// policy tests drive against the real wrapper.
#[cfg(test)]
pub struct FlakySink {
    remaining_failures: std::sync::atomic::AtomicU32,
    written_rows: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

#[cfg(test)]
impl FlakySink {
    /// Fail every write `failures` times in a row before each success; share
    /// `written_rows` so a test can assert what actually landed.
    pub fn new(failures: u32, written_rows: std::sync::Arc<std::sync::atomic::AtomicU64>) -> Self {
        Self {
            remaining_failures: std::sync::atomic::AtomicU32::new(failures),
            written_rows,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl Sink for FlakySink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        use std::sync::atomic::Ordering;
        if self.remaining_failures.load(Ordering::SeqCst) > 0 {
            self.remaining_failures.fetch_sub(1, Ordering::SeqCst);
            return Err(EngineError::Sink("transient write error".into()));
        }
        self.written_rows
            .fetch_add(batch.num_rows() as u64, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::policy::{Backoff, SinkOnError, SinkPolicy};
    use datafusion::arrow::array::{Int64Array, RecordBatch};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn batch(rows: i64) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Int64, false)]));
        let vals: Vec<i64> = (0..rows).collect();
        RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(vals))]).unwrap()
    }

    fn policy(on_error: SinkOnError, max_attempts: u32, dlq: Option<serde_json::Value>) -> SinkPolicy {
        SinkPolicy {
            on_error,
            backoff: Backoff {
                max_attempts,
                base: Duration::from_millis(1),
            },
            dlq,
        }
    }

    #[tokio::test]
    async fn retry_recovers_and_records_the_write() {
        let metrics = FlowMetrics::new();
        let landed = Arc::new(AtomicU64::new(0));
        let mut sink = MeteredSink::new(
            Box::new(FlakySink::new(2, landed.clone())),
            metrics.clone(),
            policy(SinkOnError::Halt, 5, None),
        );
        sink.write(&batch(3)).await.expect("recovers after retries");
        assert_eq!(landed.load(Ordering::SeqCst), 3, "all rows land after retry");
        let snap = metrics.snapshot();
        assert_eq!(snap.rows_written, 3);
        assert_eq!(snap.flush_count, 1);
        assert_eq!(snap.write_errors, 2, "each failed attempt is counted");
        assert!(snap.last_write_ms.is_some());
    }

    #[tokio::test]
    async fn halt_surfaces_error_without_silent_drop() {
        let landed = Arc::new(AtomicU64::new(0));
        let mut sink = MeteredSink::new(
            Box::new(FlakySink::new(10, landed.clone())),
            FlowMetrics::new(),
            policy(SinkOnError::Halt, 3, None),
        );
        assert!(sink.write(&batch(2)).await.is_err(), "halt propagates the error");
        assert_eq!(landed.load(Ordering::SeqCst), 0, "nothing landed");
    }

    #[tokio::test]
    async fn drop_continues_and_counts_errors() {
        let metrics = FlowMetrics::new();
        let landed = Arc::new(AtomicU64::new(0));
        let mut sink = MeteredSink::new(
            Box::new(FlakySink::new(10, landed.clone())),
            metrics.clone(),
            policy(SinkOnError::Drop, 3, None),
        );
        // The batch never lands but the run continues (Ok), with each attempt
        // counted so the loss is observable rather than silent.
        sink.write(&batch(2)).await.expect("drop keeps running");
        assert_eq!(landed.load(Ordering::SeqCst), 0);
        assert!(metrics.snapshot().write_errors >= 4, "all attempts counted");
    }

    #[tokio::test]
    async fn dlq_dead_letters_the_failed_batch_to_parquet() {
        let dir = std::env::temp_dir().join(format!("rw08-dlq-{}", uuid::Uuid::new_v4()));
        let dlq_cfg = serde_json::json!({
            "type": "datasource",
            "kind": "file",
            "dir": dir.to_str().unwrap(),
            "prefix": "dlq",
        });
        let landed = Arc::new(AtomicU64::new(0));
        let mut sink = MeteredSink::new(
            Box::new(FlakySink::new(10, landed.clone())),
            FlowMetrics::new(),
            policy(SinkOnError::Dlq, 2, Some(dlq_cfg)),
        );
        sink.write(&batch(4)).await.expect("dlq keeps running");
        sink.close().await.expect("dlq flushes on close");
        // A dead-letter part-file exists with the failed rows.
        let parts: Vec<_> = std::fs::read_dir(&dir)
            .expect("dlq dir created")
            .filter_map(Result::ok)
            .collect();
        assert!(!parts.is_empty(), "failed batch dead-lettered to a file");
        std::fs::remove_dir_all(&dir).ok();
    }
}
