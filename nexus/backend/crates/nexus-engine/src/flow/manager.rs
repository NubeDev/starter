//! `FlowManager` — run and stop saved ingestion flows.
//!
//! A flow is a long-lived ArkFlow `Stream` (input → pipeline → output) the
//! control plane runs on the tenant's behalf — e.g. poll weather, shape it,
//! write it to a datasource table. Unlike the live runner (one stream per
//! subscription, torn down when subscribers leave), a flow is named and runs
//! until explicitly stopped, so the manager keys running flows by id and holds
//! each one's cancellation token to stop it on demand.
//!
//! Single-node for v1, like live fan-out: the running-flow set is in-process, so
//! a flow runs on the node that started it. A scheduler/leader election for
//! multi-node flow ownership is a later concern, stated here rather than
//! discovered at deploy time.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use arkflow_core::stream::StreamConfig;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

use crate::registry::register_all;
use crate::time::now_rfc3339;

/// Per-flow run state the manager observes directly: when the current/most
/// recent run started and the error that ended the last run. Throughput is not
/// here because the manager does not see the stream's batches (those go to the
/// real output sink); reporting it would require instrumenting every sink.
#[derive(Debug, Clone, Default)]
pub struct FlowStats {
    /// RFC-3339 start time of the current or most recent run.
    pub last_started_at: Option<String>,
    /// Error that ended the most recent run, cleared on a fresh start.
    pub last_error: Option<String>,
}

/// Runs saved flows as background ArkFlow streams. Cheap to clone — the running
/// set is shared behind an `Arc`.
#[derive(Clone)]
pub struct FlowManager {
    running: Arc<Mutex<HashMap<String, CancellationToken>>>,
    /// Last-run state per flow id, retained across stop/restart so the flows
    /// list can show the last error after a run ends.
    stats: Arc<Mutex<HashMap<String, FlowStats>>>,
}

impl FlowManager {
    /// Construct a manager, ensuring the engine builders (incl. the flow
    /// connectors) are registered.
    pub fn new() -> Result<Self, String> {
        register_all()?;
        Ok(Self {
            running: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Whether the flow `id` is currently running on this node.
    pub fn is_running(&self, id: &str) -> bool {
        self.running.lock().unwrap().contains_key(id)
    }

    /// The last observed run state for `id` — start time and last error — or the
    /// default (all `None`) if the flow has never run this process.
    pub fn stats(&self, id: &str) -> FlowStats {
        self.stats.lock().unwrap().get(id).cloned().unwrap_or_default()
    }

    /// Build `{input, pipeline, output}` and spawn it as a background stream
    /// bound to a fresh cancellation token, tracked under `id`. Starting an
    /// already-running flow is a no-op (idempotent start). A build error is
    /// returned synchronously; a mid-run error ends the task and drops the flow
    /// from the running set.
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

        let config = json!({
            "input": input,
            "pipeline": { "thread_num": 1, "processors": processors },
            "output": output,
        });
        let cfg: StreamConfig =
            serde_json::from_value(config).map_err(|e| format!("invalid flow config: {e}"))?;
        let mut stream = cfg.build().map_err(|e| e.to_string())?;

        let token = CancellationToken::new();
        self.running
            .lock()
            .unwrap()
            .insert(id.to_string(), token.clone());
        // A fresh run starts clean: stamp the start time and clear the prior
        // run's error so the list reflects this run, not the last failure.
        self.stats.lock().unwrap().insert(
            id.to_string(),
            FlowStats {
                last_started_at: Some(now_rfc3339()),
                last_error: None,
            },
        );

        let running = self.running.clone();
        let stats = self.stats.clone();
        let id_owned = id.to_string();
        tokio::spawn(async move {
            if let Err(e) = stream.run(token).await {
                tracing::warn!(flow_id = %id_owned, error = %e, "flow ended with error");
                if let Some(s) = stats.lock().unwrap().get_mut(&id_owned) {
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
