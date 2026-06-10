//! `FlowManager` — run and stop saved ingestion flows.
//!
//! A flow is a long-lived native [`crate::core::Pipeline`] (input → pipeline →
//! output) the control plane runs on the tenant's behalf — e.g. poll weather,
//! shape it, write it to a datasource table. Unlike the live runner (one stream
//! per subscription, torn down when subscribers leave), a flow is named and runs
//! until explicitly stopped, so the manager keys running flows by id and holds
//! each one's cancellation token to stop it on demand.
//!
//! Each run is built against a per-flow metered registry ([`super::metered`]) so
//! its source and sink carry the run's [`FlowMetrics`] handle and the failure
//! policy parsed from the flow config. The manager retains the handle across the
//! run so the flows API can read live ingest counters without touching the
//! pipeline.
//!
//! Single-node for v1, like live fan-out: the running-flow set is in-process, so
//! a flow runs on the node that started it. A scheduler/leader election for
//! multi-node flow ownership is a later concern, stated here rather than
//! discovered at deploy time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::core::{Pipeline, PipelineConfig};
use crate::flow::metered::metered_registry;
use crate::flow::metrics::{FlowMetrics, MetricsSnapshot};
use crate::time::now_rfc3339;

/// Per-flow run state the manager observes directly: when the current/most
/// recent run started, the error that ended the last run, and the live ingest
/// counters for the current run.
#[derive(Debug, Clone, Default)]
pub struct FlowStats {
    /// RFC-3339 start time of the current or most recent run.
    pub last_started_at: Option<String>,
    /// Error that ended the most recent run, cleared on a fresh start.
    pub last_error: Option<String>,
    /// Live ingest counters for the current or most recent run.
    pub metrics: MetricsSnapshot,
}

/// Internal per-flow record: the last-run summary plus the live metrics handle
/// the run's metered nodes bump. The handle is kept so `stats` reads fresh counts
/// without coordinating with the pipeline task.
#[derive(Clone, Default)]
struct FlowState {
    last_started_at: Option<String>,
    last_error: Option<String>,
    metrics: FlowMetrics,
}

/// Runs saved flows as background native pipelines. Cheap to clone — the running
/// set is shared behind an `Arc`.
#[derive(Clone, Default)]
pub struct FlowManager {
    running: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// Per-flow run state, retained across stop/restart so the flows list can show
    /// the last error and the run's counters after a run ends.
    state: Arc<Mutex<HashMap<String, FlowState>>>,
}

impl FlowManager {
    /// Construct an empty manager. Each flow builds its own metered registry on
    /// `start`, so the manager holds no shared registry.
    pub fn new() -> Result<Self, String> {
        Ok(Self::default())
    }

    /// Whether the flow `id` is currently running on this node.
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }

    /// The last observed run state for `id` — start time, last error, and a live
    /// snapshot of its ingest counters — or the default if it has never run this
    /// process.
    pub fn stats(&self, id: &str) -> FlowStats {
        let state = self.state.lock().unwrap();
        match state.get(id) {
            Some(s) => FlowStats {
                last_started_at: s.last_started_at.clone(),
                last_error: s.last_error.clone(),
                metrics: s.metrics.snapshot(),
            },
            None => FlowStats::default(),
        }
    }

    /// Build `{input, pipeline, output}` against a per-flow metered registry and
    /// spawn it as a background stream bound to a fresh cancellation token,
    /// tracked under `id`. Starting an already-running flow is a no-op (idempotent
    /// start). A build error is returned synchronously; a mid-run error ends the
    /// task, records `last_error`, and drops the flow from the running set.
    pub fn start(
        &self,
        id: &str,
        input: Value,
        processors: Vec<Value>,
        output: Value,
    ) -> Result<(), String> {
        {
            let running = self.running.lock().unwrap();
            if running.contains_key(id) {
                return Ok(());
            }
        }

        let source_type = node_type(&input).ok_or("input node missing string \"type\"")?;
        let sink_type = node_type(&output).ok_or("output node missing string \"type\"")?;

        // A fresh metrics handle per run: the metered source/sink bump it, the API
        // reads its snapshot. Installed below before the task spawns.
        let metrics = FlowMetrics::new();
        let registry = metered_registry(&source_type, &sink_type, &input, &output, metrics.clone());

        let config = json!({
            "input": input,
            "pipeline": { "processors": processors },
            "output": output,
        });
        let cfg = PipelineConfig::from_value(config)
            .map_err(|e| format!("invalid flow config: {e}"))?;
        let pipeline = Pipeline::build(&registry, &cfg).map_err(|e| e.to_string())?;

        let token = CancellationToken::new();
        self.running
            .lock()
            .unwrap()
            .insert(id.to_string(), token.clone());
        // A fresh run starts clean: stamp the start time, clear the prior run's
        // error, and install the new metrics handle so the list reflects this run.
        self.state.lock().unwrap().insert(
            id.to_string(),
            FlowState {
                last_started_at: Some(now_rfc3339()),
                last_error: None,
                metrics,
            },
        );

        let running = self.running.clone();
        let state = self.state.clone();
        let id_owned = id.to_string();
        tokio::spawn(async move {
            if let Err(e) = pipeline.run(token).await {
                tracing::warn!(flow_id = %id_owned, error = %e, "flow ended with error");
                if let Some(s) = state.lock().unwrap().get_mut(&id_owned) {
                    s.last_error = Some(e.to_string());
                }
            }
            // Whether it ended by cancel or error, it is no longer running.
            running.lock().unwrap().remove(&id_owned);
        });
        Ok(())
    }

    /// Stop the flow `id` if running: cancel its token (which interrupts the
    /// in-flight input read and breaks the stream loop) and drop it from the
    /// running set. Returns whether a flow was actually stopped.
    pub fn stop(&self, id: &str) -> bool {
        let token = self.running.lock().unwrap().remove(id);
        match token {
            Some(t) => {
                t.cancel();
                true
            }
            None => false,
        }
    }
}

/// Read a node's `type` string from its config object.
fn node_type(node: &Value) -> Option<String> {
    node.get("type").and_then(Value::as_str).map(str::to_string)
}
