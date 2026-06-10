//! The native `sse` sink: fan each batch out to a broadcast channel for live
//! SSE subscribers.
//!
//! The native port of [`super::sse`] onto the RW-01 [`Sink`] trait. It finds its
//! broadcast channel by the `run_id` the live runner reserved (see
//! [`super::broadcast_store`]) and publishes each batch's rows through the same
//! [`broadcast_store::LiveChannel`] the ArkFlow sink uses — so the monotonic
//! sequence numbers a reconnecting client resumes from with `Last-Event-ID` are
//! assigned identically. Only the trait surface changes.

use serde::Deserialize;
use serde_json::Value;

use datafusion::arrow::array::RecordBatch;

use super::broadcast_store::{self, LiveChannel};
use crate::arrow_json::batch_to_rows;
use crate::core::{EngineError, EngineResult, Sink};

#[derive(Debug, Clone, Deserialize)]
struct SseConfig {
    run_id: String,
}

/// Publishes each batch's rows to the run's broadcast channel as the next event.
pub struct SseSink {
    channel: LiveChannel,
}

impl SseSink {
    /// Build from the node config, resolving the reserved [`LiveChannel`] by
    /// `run_id`. Returns [`EngineError::Build`] if the live runner did not
    /// reserve a channel for that id first.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: SseConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid sse config: {e}")))?;
        let channel = broadcast_store::lookup(&config.run_id)
            .ok_or_else(|| EngineError::Build(format!("unknown sse run_id: {}", config.run_id)))?;
        Ok(Self { channel })
    }
}

#[async_trait::async_trait]
impl Sink for SseSink {
    async fn write(&mut self, batch: &RecordBatch) -> EngineResult<()> {
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let converted = batch_to_rows(batch).map_err(EngineError::Sink)?;
        // A publish with no subscribers is fine — the stream stays warm. Per-
        // subscriber lag is handled by the broadcast receiver, not here.
        self.channel.publish(converted.rows);
        Ok(())
    }

    async fn close(&mut self) -> EngineResult<()> {
        Ok(())
    }
}
