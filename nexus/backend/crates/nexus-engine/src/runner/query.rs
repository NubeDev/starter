//! `QueryRunner` — drive a finite native pipeline once and hand back its rows.
//!
//! The caller supplies an input config and a processor chain; the runner swaps
//! in a bounded `collector` output, runs the pipeline to completion (or to the
//! first breached cap / the wall-clock deadline), and drains the collected rows.
//! This is the one-shot query path behind `POST /query`.

use std::sync::Arc;
use std::time::Instant;

use nexus_spi::dto::query::{ColumnSchema, QueryStats};
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use super::cancel;
use crate::core::{Pipeline, PipelineConfig, Registry};
use crate::native_registry;
use crate::sink::cap::Caps;
use crate::sink::store;

/// A completed one-shot query.
pub struct QueryOutcome {
    pub columns: Vec<ColumnSchema>,
    pub rows: Vec<Value>,
    pub stats: QueryStats,
}

/// Runs one-shot queries through the bounded collector. Cheap to clone/share —
/// the node registry sits behind an `Arc`; each `run` reserves its own collector
/// buffer.
#[derive(Clone)]
pub struct QueryRunner {
    registry: Arc<Registry>,
}

impl QueryRunner {
    /// Construct a runner holding the native node registry. The registry is built
    /// once per runner and shared across every `run`.
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            registry: Arc::new(native_registry()),
        })
    }

    /// Run `input` through `processors` and collect the result, bounded by
    /// `caps`. The `output` is always the collector, so any `output` in the
    /// caller's config is irrelevant.
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
        let result = build_and_run(&self.registry, &run_id, input, processors, caps, token).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        let drained = store::take(&run_id);

        // A cancellation fired by a breached cap is an expected stop, not a
        // failure — the result stands and is reported truncated. Any other
        // engine error with no collected rows is a real failure.
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
    registry: &Registry,
    run_id: &str,
    input: Value,
    processors: Vec<Value>,
    caps: Caps,
    token: CancellationToken,
) -> Result<(), String> {
    let config = json!({
        "input": input,
        "pipeline": { "processors": processors },
        "output": { "type": "collector", "run_id": run_id },
    });

    let cfg = PipelineConfig::from_value(config).map_err(|e| e.to_string())?;
    let pipeline = Pipeline::build(registry, &cfg).map_err(|e| e.to_string())?;

    let timer = cancel::deadline(token.clone(), caps.max_duration);
    let result = pipeline.run(token).await;
    if let Some(timer) = timer {
        timer.abort();
    }
    result.map(|_| ()).map_err(|e| e.to_string())
}
