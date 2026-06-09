//! A custom ArkFlow output (`type: sse`) that fans each Arrow batch out to a
//! `tokio::broadcast` channel for live SSE subscribers.
//!
//! The live counterpart to the collector: where the collector buffers a finite
//! result, this forwards every batch of an unbounded stream to all current
//! subscribers as a [`StreamEvent`] carrying a monotonic sequence number (so a
//! reconnecting client can resume with `Last-Event-ID`). Like the collector it
//! is built from a config `Value`, so it finds its broadcast channel by the
//! `run_id` the live runner reserved (see [`super::broadcast_store`]).

use std::sync::Arc;

use arkflow_core::codec::Codec;
use arkflow_core::output::{register_output_builder, Output, OutputBuilder};
use arkflow_core::{Error, MessageBatchRef, Resource};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use super::broadcast_store::{self, LiveChannel};
use crate::arrow_json;

#[derive(Debug, Clone, Deserialize)]
struct SseConfig {
    run_id: String,
}

struct SseOutput {
    channel: LiveChannel,
}

#[async_trait]
impl Output for SseOutput {
    async fn connect(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn write(&self, msg: MessageBatchRef) -> Result<(), Error> {
        let batch = &**msg;
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let converted = arrow_json::batch_to_rows(batch).map_err(Error::Process)?;
        // A send with no subscribers is fine — the stream stays warm for the
        // next subscriber. Backpressure/lag is handled per-subscriber by the
        // broadcast receiver, not here.
        self.channel.publish(converted.rows);
        Ok(())
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct SseOutputBuilder;

impl OutputBuilder for SseOutputBuilder {
    fn build(
        &self,
        _name: Option<&String>,
        config: &Option<Value>,
        _codec: Option<Arc<dyn Codec>>,
        _resource: &Resource,
    ) -> Result<Arc<dyn Output>, Error> {
        let config: SseConfig = config
            .clone()
            .ok_or_else(|| Error::Config("sse output requires a run_id".into()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| Error::Config(format!("invalid sse config: {e}")))
            })?;
        let channel = broadcast_store::lookup(&config.run_id)
            .ok_or_else(|| Error::Config(format!("unknown sse run_id: {}", config.run_id)))?;
        Ok(Arc::new(SseOutput { channel }))
    }
}

/// Register the `sse` output type. Called once at startup.
pub fn init() -> Result<(), Error> {
    register_output_builder("sse", Arc::new(SseOutputBuilder))
}
