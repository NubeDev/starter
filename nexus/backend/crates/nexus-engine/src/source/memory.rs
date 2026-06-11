//! The native `memory` source: replay a fixed list of JSON-document strings,
//! one batch each, then end.
//!
//! The finite test/dry-run input behind the one-shot query seam — a `memory →
//! json_to_arrow → sql → collector` chain exercises the exact path a real
//! datasource source will, with known rows and no external dependency. Each
//! configured `messages` string is emitted as a one-document carrier batch (see
//! [`crate::arrow_json::JSON_VALUE_FIELD`]); once they are exhausted `read`
//! returns `None`, so the pipeline completes cleanly.

use std::collections::VecDeque;

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};

#[derive(Debug, Clone, Deserialize)]
struct MemoryConfig {
    /// The JSON-document strings to replay, in order. Each becomes one batch.
    #[serde(default)]
    messages: Vec<String>,
}

/// Replays a finite queue of JSON documents as carrier batches.
pub struct MemorySource {
    queue: VecDeque<String>,
}

impl MemorySource {
    /// Build from the node config. An absent or empty `messages` list yields a
    /// source that ends immediately on its first `read`.
    pub fn from_config(config: &Value) -> EngineResult<Self> {
        let config: MemoryConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid memory config: {e}")))?;
        Ok(Self {
            queue: config.messages.into(),
        })
    }
}

#[async_trait::async_trait]
impl Source for MemorySource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        match self.queue.pop_front() {
            Some(doc) => Ok(Some(json_carrier_batch(&[doc]))),
            None => Ok(None),
        }
    }
}
