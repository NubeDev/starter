//! Process-global registry mapping a run id to the bounded buffer its collector
//! sink appends to.
//!
//! The collector sink is built from a config `Value`, so the only way to connect
//! a running sink to the runner that wants its rows is out of band: the runner
//! reserves a [`RunSink`] under a fresh id, writes that id into the output
//! config, and drains the same id once the run finishes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::cap::{CapState, Caps};
use crate::arrow_json::JsonBatch;
use nexus_spi::dto::query::ColumnSchema;

/// The shared, bounded buffer for one run. Cloning shares the same inner state
/// (the sink and the runner hold clones). Writes are accounted against [`Caps`];
/// the first over-cap batch trips `truncated` and cancels the run's token.
#[derive(Clone)]
pub struct RunSink {
    inner: Arc<Mutex<Inner>>,
    token: CancellationToken,
}

struct Inner {
    caps: Caps,
    state: CapState,
    columns: Vec<ColumnSchema>,
    rows: Vec<Value>,
}

impl RunSink {
    fn new(caps: Caps, token: CancellationToken) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                caps,
                state: CapState::default(),
                columns: Vec::new(),
                rows: Vec::new(),
            })),
            token,
        }
    }

    /// Append a converted batch if it fits within the caps; otherwise drop it,
    /// mark the run truncated, and cancel the stream so it stops producing.
    pub fn absorb(&self, columns: Vec<ColumnSchema>, batch: JsonBatch) {
        let mut inner = self.inner.lock().unwrap();
        let rows = batch.rows.len() as u64;
        let caps = inner.caps;
        if !inner.state.admit(rows, batch.bytes, &caps) {
            // Over a cap: stop the stream. Already-collected rows stand; the
            // result is reported truncated.
            self.token.cancel();
            return;
        }
        if inner.columns.is_empty() {
            inner.columns = columns;
        }
        inner.rows.extend(batch.rows);
    }
}

/// What a finished run yields when drained.
pub struct Drained {
    pub columns: Vec<ColumnSchema>,
    pub rows: Vec<Value>,
    pub bytes: u64,
    pub truncated: bool,
}

fn registry() -> &'static Mutex<HashMap<String, RunSink>> {
    static RUNS: OnceLock<Mutex<HashMap<String, RunSink>>> = OnceLock::new();
    RUNS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Reserve a fresh bounded buffer for `run_id` before the stream starts. The
/// `token` is cancelled by the sink if a cap is breached, and is the same token
/// the runner passes to `Stream::run`.
pub fn open(run_id: &str, caps: Caps, token: CancellationToken) -> RunSink {
    let sink = RunSink::new(caps, token);
    registry()
        .lock()
        .unwrap()
        .insert(run_id.to_string(), sink.clone());
    sink
}

/// Look up the buffer a running sink should append to.
pub fn lookup(run_id: &str) -> Option<RunSink> {
    registry().lock().unwrap().get(run_id).cloned()
}

/// Drain and remove the buffer once the run has finished.
pub fn take(run_id: &str) -> Drained {
    let removed = registry().lock().unwrap().remove(run_id);
    match removed {
        Some(sink) => {
            let mut inner = sink.inner.lock().unwrap();
            Drained {
                columns: std::mem::take(&mut inner.columns),
                rows: std::mem::take(&mut inner.rows),
                bytes: inner.state.bytes,
                truncated: inner.state.truncated,
            }
        }
        None => Drained {
            columns: Vec::new(),
            rows: Vec::new(),
            bytes: 0,
            truncated: false,
        },
    }
}
