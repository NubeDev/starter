//! The native `generate` source: emit a fixed JSON document on an interval, for
//! tests and dry runs.
//!
//! Mirrors ArkFlow's `generate` input: the first read fires immediately, later
//! reads wait `interval`; each read emits `batch_size` copies of the `context`
//! document as one carrier batch. An optional `count` bounds the total documents
//! — once reached `read` returns `None`, so a finite flow test terminates; with
//! no `count` the source runs until its cancellation token fires.

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};
use crate::source::interval::parse_cadence;

#[derive(Debug, Clone, Deserialize)]
struct GenerateConfig {
    /// The JSON document emitted on every tick.
    context: String,
    /// Delay between ticks, e.g. `"5ms"`, `"1s"`.
    interval: String,
    /// Documents per tick (default 1).
    #[serde(default)]
    batch_size: Option<usize>,
    /// Total documents to emit before ending; unbounded when absent.
    #[serde(default)]
    count: Option<usize>,
}

/// Emits a fixed document on a fixed cadence, optionally bounded by a total
/// count.
pub struct GenerateSource {
    context: String,
    interval: std::time::Duration,
    batch_size: usize,
    count: Option<usize>,
    emitted: usize,
    first: bool,
}

impl GenerateSource {
    /// Build from the node config, requiring `context` and `interval`. A
    /// `batch_size` of zero is treated as one so the source always makes
    /// progress.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: GenerateConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid generate config: {e}")))?;
        let interval = parse_cadence(&config.interval)
            .map_err(|e| EngineError::Build(format!("invalid generate interval: {e}")))?;
        Ok(Self {
            context: config.context,
            interval,
            batch_size: config.batch_size.unwrap_or(1).max(1),
            count: config.count,
            emitted: 0,
            first: true,
        })
    }
}

#[async_trait::async_trait]
impl Source for GenerateSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        if let Some(count) = self.count {
            // End before a partial batch would overshoot the requested count,
            // matching ArkFlow's all-or-nothing batch semantics.
            if self.emitted + self.batch_size > count {
                return Ok(None);
            }
        }
        if self.first {
            self.first = false;
        } else {
            tokio::time::sleep(self.interval).await;
        }
        let docs = vec![self.context.clone(); self.batch_size];
        self.emitted += self.batch_size;
        Ok(Some(json_carrier_batch(&docs)))
    }
}
