//! `QueryRunner` — drive a finite ArkFlow `Stream` once and hand back its rows.
//!
//! The caller supplies an input config and a processor chain; the runner swaps
//! in a bounded `collector` output, runs the stream to completion (or to the
//! first breached cap / the wall-clock deadline), and drains the collected rows.
//! This is the one-shot query path behind `POST /query`.

use std::time::Instant;

use arkflow_core::stream::StreamConfig;
use nexus_spi::dto::query::{ColumnSchema, QueryStats};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::cancel;
use crate::registry::register_all;
use crate::sink::cap::Caps;
use crate::sink::store;

/// A completed one-shot query.
pub struct QueryOutcome {
    pub columns: Vec<ColumnSchema>,
    pub rows: Vec<Value>,
    pub stats: QueryStats,
}

/// Runs one-shot queries through the bounded collector. Cheap to clone/share;
/// holds no per-query state — each `run` reserves its own collector buffer.
#[derive(Clone)]
pub struct QueryRunner {
    _private: (),
}

impl QueryRunner {
    /// Construct a runner, ensuring the engine builders are registered. Safe to
    /// call repeatedly — registration happens once per process.
    pub fn new() -> Result<Self, String> {
        register_all()?;
        Ok(Self { _private: () })
    }

    /// Run `input` through `processors` and collect the result, bounded by
    /// `caps`. The `output` is always replaced with the collector, so any
    /// `output` in the caller's config is irrelevant.
    pub async fn run(
        &self,
        input: Value,
        processors: Vec<Value>,
        caps: Caps,
    ) -> Result<QueryOutcome, String> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let token = CancellationToken::new();
        store::open(&run_id, caps, token.clone());

        let started = Instant::now();
        let result = build_and_run(&run_id, input, processors, caps, token).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let drained = store::take(&run_id);

        // A cancellation fired by a breached cap is an expected stop, not a
        // failure — the result stands and is reported truncated. Any other
        // stream error with no collected rows is a real failure.
        if let Err(e) = result {
            if !drained.truncated && drained.rows.is_empty() {
                return Err(e);
            }
        }

        let row_count = drained.rows.len() as u64;
        Ok(QueryOutcome {
            columns: drained.columns,
            rows: drained.rows,
            stats: QueryStats {
                row_count,
                byte_count: drained.bytes,
                elapsed_ms,
                truncated: drained.truncated,
            },
        })
    }
}

async fn build_and_run(
    run_id: &str,
    input: Value,
    processors: Vec<Value>,
    caps: Caps,
    token: CancellationToken,
) -> Result<(), String> {
    let config = json!({
        "input": input,
        "pipeline": { "thread_num": 1, "processors": processors },
        "output": { "type": "collector", "run_id": run_id },
    });

    let cfg: StreamConfig =
        serde_json::from_value(config).map_err(|e| format!("invalid stream config: {e}"))?;
    let mut stream = cfg.build().map_err(|e| e.to_string())?;

    let timer = cancel::deadline(token.clone(), caps.max_duration);
    let result = stream.run(token).await;
    if let Some(timer) = timer {
        timer.abort();
    }
    result.map_err(|e| e.to_string())
}
