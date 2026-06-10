//! A `Source` wrapper that counts batches and applies the flow's read-error policy.
//!
//! Wraps the real source the registry built so the bounded-channel pipeline drives
//! it unchanged. On a successful read it bumps the flow's batch counter; on a read
//! error it applies `source_on_error`: retry with capped backoff, then either halt
//! the run (surface the error) or, when retries are exhausted under
//! `retry_backoff`, surface the error so the flow enters its `last_error` state —
//! retries that never succeed must not loop forever (roadmap §6 source policy).
//!
//! `commit` is forwarded verbatim so the "no silent data loss" upstream-ack
//! contract still holds through the wrapper.

use async_trait::async_trait;
use datafusion::arrow::array::RecordBatch;

#[cfg(test)]
use crate::core::EngineError;
use crate::core::{EngineResult, Source};
use crate::flow::metrics::FlowMetrics;
use crate::flow::policy::{SourceOnError, SourcePolicy};

/// Decorates a source with per-flow batch counting and read-error retry/backoff.
pub struct MeteredSource {
    inner: Box<dyn Source>,
    metrics: FlowMetrics,
    policy: SourcePolicy,
}

impl MeteredSource {
    /// Wrap `inner` with the flow's `metrics` handle and read-error `policy`.
    pub fn new(inner: Box<dyn Source>, metrics: FlowMetrics, policy: SourcePolicy) -> Self {
        Self {
            inner,
            metrics,
            policy,
        }
    }
}

#[async_trait]
impl Source for MeteredSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        let mut attempt = 0u32;
        loop {
            match self.inner.read().await {
                Ok(Some(batch)) => {
                    self.metrics.record_batch_in();
                    return Ok(Some(batch));
                }
                Ok(None) => return Ok(None),
                Err(e) => {
                    // `halt` stops immediately; `retry_backoff` retries up to the
                    // cap, then surfaces the last error (the run halts on it, never
                    // a silent infinite loop).
                    if self.policy.on_error == SourceOnError::Halt
                        || attempt >= self.policy.backoff.max_attempts
                    {
                        return Err(e);
                    }
                    attempt += 1;
                    tracing::warn!(
                        attempt,
                        error = %e,
                        "flow source read failed; retrying after backoff"
                    );
                    tokio::time::sleep(self.policy.backoff.delay_for(attempt)).await;
                }
            }
        }
    }

    async fn commit(&mut self) -> EngineResult<()> {
        self.inner.commit().await
    }
}

/// A source whose `read` errors `count` times before succeeding — the seam the
/// retry tests drive. Lives here so the policy behaviour is tested against the
/// real wrapper, not a hand-rolled double in the test crate.
#[cfg(test)]
pub struct FlakySource {
    remaining_failures: u32,
    batch: Option<RecordBatch>,
}

#[cfg(test)]
impl FlakySource {
    /// Fail `failures` times, then yield `batch` once, then end the stream.
    pub fn new(failures: u32, batch: RecordBatch) -> Self {
        Self {
            remaining_failures: failures,
            batch: Some(batch),
        }
    }
}

#[cfg(test)]
#[async_trait]
impl Source for FlakySource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        if self.remaining_failures > 0 {
            self.remaining_failures -= 1;
            return Err(EngineError::Source("transient read error".into()));
        }
        Ok(self.batch.take())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::policy::{Backoff, SourceOnError, SourcePolicy};
    use datafusion::arrow::array::{Int64Array, RecordBatch};
    use datafusion::arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;
    use std::time::Duration;

    fn one_row_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new("v", DataType::Int64, false)]));
        RecordBatch::try_new(schema, vec![Arc::new(Int64Array::from(vec![1]))]).unwrap()
    }

    fn fast_backoff(max_attempts: u32) -> Backoff {
        Backoff {
            max_attempts,
            base: Duration::from_millis(1),
        }
    }

    #[tokio::test]
    async fn retry_backoff_recovers_then_counts_the_batch() {
        let metrics = FlowMetrics::new();
        let policy = SourcePolicy {
            on_error: SourceOnError::RetryBackoff,
            backoff: fast_backoff(5),
        };
        let mut src = MeteredSource::new(
            Box::new(FlakySource::new(2, one_row_batch())),
            metrics.clone(),
            policy,
        );
        // First read fails twice, then yields the batch.
        let got = src.read().await.expect("recovers after retries");
        assert!(got.is_some(), "the batch is delivered after backoff");
        assert_eq!(metrics.snapshot().batches_in, 1);
        // Stream then ends cleanly.
        assert!(src.read().await.expect("clean end").is_none());
    }

    #[tokio::test]
    async fn halt_surfaces_the_first_error() {
        let policy = SourcePolicy {
            on_error: SourceOnError::Halt,
            backoff: fast_backoff(5),
        };
        let mut src =
            MeteredSource::new(Box::new(FlakySource::new(1, one_row_batch())), FlowMetrics::new(), policy);
        assert!(src.read().await.is_err(), "halt does not retry");
    }

    #[tokio::test]
    async fn retry_backoff_gives_up_after_the_cap() {
        let policy = SourcePolicy {
            on_error: SourceOnError::RetryBackoff,
            backoff: fast_backoff(2),
        };
        // Fails more times than the cap allows — the run halts on the error rather
        // than looping forever.
        let mut src =
            MeteredSource::new(Box::new(FlakySource::new(10, one_row_batch())), FlowMetrics::new(), policy);
        assert!(src.read().await.is_err(), "exhausted retries surface the error");
    }
}
