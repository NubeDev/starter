//! `LiveRunner` — drive an unbounded ArkFlow `Stream` into the SSE broadcast.
//!
//! The live counterpart to `QueryRunner`. Where the query runner runs a finite
//! stream to completion and drains it, the live runner spawns a never-ending
//! stream as a background task whose output is the `sse` sink; cancelling the
//! token (when the last subscriber leaves, via the stream registry) stops it.

use arkflow_core::stream::StreamConfig;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::registry::register_all;

/// Spawns unbounded streams that publish to the SSE broadcast channels. Cheap to
/// clone; holds no per-stream state.
#[derive(Clone)]
pub struct LiveRunner {
    _private: (),
}

impl LiveRunner {
    /// Construct a runner, ensuring the engine builders are registered.
    pub fn new() -> Result<Self, String> {
        register_all()?;
        Ok(Self { _private: () })
    }

    /// Build `{input, pipeline, output: sse(run_id)}` and spawn it as a
    /// background task bound to `token`. Returns once the stream is spawned;
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
            "pipeline": { "thread_num": 1, "processors": processors },
            "output": { "type": "sse", "run_id": run_id },
        });
        let cfg: StreamConfig =
            serde_json::from_value(config).map_err(|e| format!("invalid stream config: {e}"))?;
        let mut stream = cfg.build().map_err(|e| e.to_string())?;

        tokio::spawn(async move {
            if let Err(e) = stream.run(token).await {
                tracing::warn!(error = %e, "live stream ended with error");
            }
        });
        Ok(())
    }
}
