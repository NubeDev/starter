//! Shared sink for one pipeline run — maps a run id to the rows it collected.
//!
//! The collector output (see [`super::sink`]) can only hand results back to the
//! HTTP handler out-of-band, because ArkFlow builds it from a config `Value`.
//! We bridge that gap with a process-global registry keyed by run id.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use once_cell::sync::Lazy;
use serde_json::Value;

/// Rows collected by a single run, shared between the sink and the handler.
pub type RunRows = Arc<Mutex<Vec<Value>>>;

static RUNS: Lazy<Mutex<HashMap<String, RunRows>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Reserve a fresh, empty buffer for `run_id` before the stream starts.
pub fn open(run_id: &str) -> RunRows {
    let rows: RunRows = Arc::new(Mutex::new(Vec::new()));
    RUNS.lock().unwrap().insert(run_id.to_string(), rows.clone());
    rows
}

/// Look up the buffer a running sink should append to.
pub fn lookup(run_id: &str) -> Option<RunRows> {
    RUNS.lock().unwrap().get(run_id).cloned()
}

/// Drain and remove the buffer once the run has finished.
pub fn take(run_id: &str) -> Vec<Value> {
    let removed = RUNS.lock().unwrap().remove(run_id);
    removed
        .map(|rows| std::mem::take(&mut *rows.lock().unwrap()))
        .unwrap_or_default()
}
