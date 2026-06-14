//! Run a stream config to completion (or timeout), capturing its rows.
//!
//! The submitted output is replaced with the in-memory `collector` so results
//! come back in-process. A `generate` input runs forever, so every run is
//! bounded by a cancellation token fired after `timeout_ms`.

use std::time::Duration;

use arkflow_core::stream::StreamConfig;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::collector;

/// The outcome of one bounded run.
pub struct RunOutcome {
    pub rows: Vec<Value>,
    pub error: Option<String>,
    pub cancelled: bool,
}

/// Replace the output with a collector, build the stream, and run it bounded by
/// `timeout_ms`. Always drains the collector buffer, even on error.
pub async fn run_config(mut config: Value, timeout_ms: u64) -> RunOutcome {
    let run_id = uuid::Uuid::new_v4().to_string();
    config["output"] = json!({ "type": "collector", "run_id": run_id });

    collector::open(&run_id);
    let outcome = build_and_run(config, timeout_ms).await;
    let rows = collector::take(&run_id);

    RunOutcome {
        rows,
        error: outcome.error,
        cancelled: outcome.cancelled,
    }
}

struct Bounded {
    error: Option<String>,
    cancelled: bool,
}

async fn build_and_run(config: Value, timeout_ms: u64) -> Bounded {
    let cfg: StreamConfig = match serde_json::from_value(config) {
        Ok(cfg) => cfg,
        Err(e) => {
            return Bounded {
                error: Some(format!("invalid config: {e}")),
                cancelled: false,
            }
        }
    };

    let mut stream = match cfg.build() {
        Ok(s) => s,
        Err(e) => {
            return Bounded {
                error: Some(e.to_string()),
                cancelled: false,
            }
        }
    };

    let token = CancellationToken::new();
    let guard = token.clone();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
        guard.cancel();
    });

    let result = stream.run(token.clone()).await;
    let cancelled = token.is_cancelled();
    timer.abort();

    Bounded {
        error: result.err().map(|e| e.to_string()),
        cancelled,
    }
}
