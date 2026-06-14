//! A custom ArkFlow output (`type: collector`) that captures rows in memory.
//!
//! Built-in outputs send data to stdout/Kafka/etc.; for a UI we need the rows
//! back in-process. This sink converts each Arrow batch to JSON and appends it
//! to the run buffer keyed by `run_id` (see [`super::store`]).

use std::sync::Arc;

use arkflow_core::codec::Codec;
use arkflow_core::output::{register_output_builder, Output, OutputBuilder};
use arkflow_core::{Error, MessageBatchRef, Resource};
use async_trait::async_trait;
use datafusion::arrow::json::ArrayWriter;
use serde::Deserialize;
use serde_json::Value;

use super::store::{self, RunRows};

#[derive(Debug, Clone, Deserialize)]
struct CollectorConfig {
    run_id: String,
}

struct CollectorOutput {
    rows: RunRows,
}

#[async_trait]
impl Output for CollectorOutput {
    async fn connect(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn write(&self, msg: MessageBatchRef) -> Result<(), Error> {
        let mut buf = Vec::new();
        let mut writer = ArrayWriter::new(&mut buf);
        writer
            .write(&msg)
            .map_err(|e| Error::Process(format!("collector arrow->json failed: {e}")))?;
        writer
            .finish()
            .map_err(|e| Error::Process(format!("collector arrow->json finish failed: {e}")))?;

        let parsed: Vec<Value> = serde_json::from_slice(&buf)?;
        self.rows.lock().unwrap().extend(parsed);
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

        let rows = store::lookup(&config.run_id)
            .ok_or_else(|| Error::Config(format!("unknown collector run_id: {}", config.run_id)))?;
        Ok(Arc::new(CollectorOutput { rows }))
    }
}

/// Register the `collector` output type. Called once at startup.
pub fn init() -> Result<(), Error> {
    register_output_builder("collector", Arc::new(CollectorOutputBuilder))
}
