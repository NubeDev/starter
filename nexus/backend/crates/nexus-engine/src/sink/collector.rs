//! A custom ArkFlow output (`type: collector`) that captures rows in memory,
//! bounded by per-run caps.
//!
//! Built-in outputs send data to stdout/Kafka/etc.; a request/response product
//! needs the rows back in process. ArkFlow builds an output from a config
//! `Value`, so the sink cannot be handed its destination buffer directly — it
//! looks the buffer up by a `run_id` the runner reserved beforehand (see
//! [`super::store`]). Each write is accounted against the run's [`Caps`]; the
//! first batch that would breach a cap is dropped, the run is flagged truncated,
//! and the run's cancellation token is fired so the stream stops promptly rather
//! than continuing to produce rows that would be discarded.

use std::sync::Arc;

use arkflow_core::codec::Codec;
use arkflow_core::output::{register_output_builder, Output, OutputBuilder};
use arkflow_core::{Error, MessageBatchRef, Resource};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use super::store::{self, RunSink};
use crate::arrow_json;

#[derive(Debug, Clone, Deserialize)]
struct CollectorConfig {
    run_id: String,
}

struct CollectorOutput {
    sink: RunSink,
}

#[async_trait]
impl Output for CollectorOutput {
    async fn connect(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn write(&self, msg: MessageBatchRef) -> Result<(), Error> {
        // `MessageBatch` derefs to the underlying Arrow `RecordBatch`.
        let batch = &**msg;
        if batch.num_rows() == 0 {
            return Ok(());
        }
        let converted = arrow_json::batch_to_rows(batch).map_err(Error::Process)?;
        let columns = arrow_json::columns_of(batch);
        self.sink.absorb(columns, converted);
        Ok(())
    }

    async fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

struct CollectorOutputBuilder;

impl OutputBuilder for CollectorOutputBuilder {
    fn build(
        &self,
        _name: Option<&String>,
        config: &Option<Value>,
        _codec: Option<Arc<dyn Codec>>,
        _resource: &Resource,
    ) -> Result<Arc<dyn Output>, Error> {
        let config: CollectorConfig = config
            .clone()
            .ok_or_else(|| Error::Config("collector output requires a run_id".into()))
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|e| Error::Config(format!("invalid collector config: {e}")))
            })?;

        let sink = store::lookup(&config.run_id)
            .ok_or_else(|| Error::Config(format!("unknown collector run_id: {}", config.run_id)))?;
        Ok(Arc::new(CollectorOutput { sink }))
    }
}

/// Register the `collector` output type. Called once at startup by
/// [`crate::registry::register_all`]; registering twice is an error.
pub fn init() -> Result<(), Error> {
    register_output_builder("collector", Arc::new(CollectorOutputBuilder))
}
