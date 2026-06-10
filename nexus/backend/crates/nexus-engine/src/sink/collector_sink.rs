//! The native `collector` sink: capture rows in memory for a one-shot run,
//! bounded by per-run caps.
//!
//! The native port of [`super::collector`] onto the RW-01 [`Sink`] trait. It
//! looks its bounded buffer up by the `run_id` the runner reserved (see
//! [`super::store`]) and absorbs each batch through the same [`store::RunSink`]
//! the ArkFlow collector uses — so caps, the truncation flag, and the run-token
//! cancellation on a breached cap stay bit-identical. The only change is the
//! trait surface; the accounting is untouched.

use serde::Deserialize;
use serde_json::Value;

use datafusion::arrow::array::RecordBatch;

use super::store::{self, RunSink};
use crate::arrow_json::{batch_to_rows, columns_of};
use crate::core::{EngineError, EngineResult, Sink};

#[derive(Debug, Clone, Deserialize)]
struct CollectorConfig {
    run_id: String,
}

/// Appends each batch to the run's bounded buffer, dropping and flagging the
/// first over-cap batch.
pub struct CollectorSink {
    sink: RunSink,
}

impl CollectorSink {
    /// Build from the node config, resolving the reserved [`RunSink`] by
    /// `run_id`. Returns [`EngineError::Build`] if the runner did not reserve a
    /// buffer for that id first.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: CollectorConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid collector config: {e}")))?;
        let sink = store::lookup(&config.run_id).ok_or_else(|| {
            EngineError::Build(format!("unknown collector run_id: {}", config.run_id))
        })?;
        Ok(Self { sink })
    }
}

#[async_trait::async_trait]
impl Sink for CollectorSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let converted = batch_to_rows(batch).map_err(EngineError::Sink)?;
        let columns = columns_of(batch);
        self.sink.absorb(columns, converted);
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        Ok(())
    }
}
