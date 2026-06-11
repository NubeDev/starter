//! `LiveRunner` — drive an unbounded native pipeline into the SSE broadcast.
//!
//! The live counterpart to `QueryRunner`. Where the query runner runs a finite
//! pipeline to completion and drains it, the live runner spawns a never-ending
//! pipeline as a background task whose output is the `sse` sink; cancelling the
//! token (when the last subscriber leaves, via the stream registry) stops it.

use std::sync::Arc;

use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::core::{Pipeline, PipelineConfig, Registry};
use crate::native_registry;

/// Spawns unbounded pipelines that publish to the SSE broadcast channels. Cheap
/// to clone — the node registry sits behind an `Arc`.
#[derive(Clone)]
pub struct LiveRunner {
    registry: Arc<Registry>,
}

impl LiveRunner {
    /// Construct a runner holding the native node registry.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            registry: Arc::new(native_registry()),
        })
    }

    /// Build `{input, pipeline, output: sse(run_id)}` and spawn it as a
    /// background task bound to `token`. Returns once the pipeline is spawned;
    /// rows then flow to the `run_id` channel until the token is cancelled. A
    /// build error is returned synchronously; a mid-run error is logged and
    /// ends the task (subscribers see the channel close).
    pub fn spawn(
        &self,
        input: Value,
        processors: Vec<Value>,
        run_id: &str,
        token: CancellationToken,
    ) -> Result<(), String> {
        let config = json!({
            "input": input,
            "pipeline": { "processors": processors },
            "output": { "type": "sse", "run_id": run_id },
        });
        let cfg = PipelineConfig::from_value(config).map_err(|e| e.to_string())?;
        let pipeline = Pipeline::build(&self.registry, &cfg).map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            if let Err(e) = pipeline.run(token).await {
                tracing::warn!(error = %e, "live stream ended with error");
            }
        });
        Ok(())
    }
}
