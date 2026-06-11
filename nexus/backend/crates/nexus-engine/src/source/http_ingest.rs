//! The native `http_ingest` source: a push input fed by an out-of-band HTTP
//! handler rather than by polling an upstream.
//!
//! Where `http_poll` pulls a remote endpoint on a cadence, `http_ingest` is the
//! converse — the transport (a REST route, or an extension's `ingest.write`)
//! pushes JSON documents *into* the running flow. The source owns the consuming
//! end of a bounded channel; the push side looks the flow up in a shared
//! [`IngestChannels`] registry and tries to enqueue. A full channel is the
//! backpressure signal the caller surfaces (HTTP `429 + Retry-After`): the bound
//! is the same bounded-channel mechanism the pipeline already uses downstream,
//! lifted to the ingest boundary so a fast pusher cannot outrun the sink.
//!
//! The registry is keyed by flow id so one process can run many push flows at
//! once; the source registers its sender on build and removes it on drop, so a
//! stopped flow stops accepting pushes (a push to an absent flow is "not
//! running", which the route maps to a 404).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use datafusion::arrow::array::RecordBatch;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::arrow_json::json_carrier_batch;
use crate::core::{EngineError, EngineResult, Source};

/// Default bounded-channel capacity (batches in flight) when a flow's
/// `http_ingest` config omits one. Small by design — the channel is the
/// backpressure boundary, so the default favours a prompt `429` over a deep
/// queue that hides a slow sink.
pub const DEFAULT_INGEST_CAPACITY: usize = 256;

/// One pushed unit of work: the JSON documents from a single push call, carried
/// to the source as one batch. An empty push is never enqueued by the route, so
/// every received item yields at least one row.
pub type IngestDocs = Vec<String>;

/// Why a push could not be enqueued, returned to the transport so it can choose
/// the right HTTP status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IngestError {
    /// No flow with this id is currently accepting pushes (not running, or not an
    /// `http_ingest` flow). The route maps this to `404`.
    NotRunning,
    /// The flow is running but its channel is full. The route maps this to `429`
    /// with a `Retry-After`. Carries the suggested back-off in seconds.
    Full { retry_after_secs: u64 },
}

/// Shared, process-wide map from flow id to the sender feeding that flow's
/// `http_ingest` source. Cheap to clone (an `Arc`); the [`FlowManager`] holds one
/// and hands it to both the per-flow source builder and the push route.
///
/// [`FlowManager`]: crate::flow::FlowManager
#[derive(Clone, Default)]
pub struct IngestChannels {
    senders: Arc<Mutex<HashMap<String, mpsc::Sender<IngestDocs>>>>,
}

impl IngestChannels {
    /// An empty registry. The flow manager creates one and shares clones.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a flow is currently registered to accept pushes.
    pub fn is_open(&self, flow_id: &str) -> bool {
        self.senders.lock().unwrap().contains_key(flow_id)
    }

    /// Try to enqueue one push's documents for `flow_id` without blocking.
    ///
    /// `try_send` never waits: a full channel returns [`IngestError::Full`] so the
    /// caller can answer `429` immediately rather than holding the request open.
    /// An unknown flow returns [`IngestError::NotRunning`]. `retry_after_secs` is
    /// the constant back-off hint the push contract documents.
    pub fn try_push(&self, flow_id: &str, docs: IngestDocs) -> Result<(), IngestError> {
        let sender = {
            let map = self.senders.lock().unwrap();
            map.get(flow_id).cloned()
        };
        let sender = sender.ok_or(IngestError::NotRunning)?;
        sender.try_send(docs).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => IngestError::Full {
                retry_after_secs: RETRY_AFTER_SECS,
            },
            // The receiver is gone (flow stopped between the lookup and the send);
            // treat it as not running so a racing push gets the 404, not a 500.
            mpsc::error::TrySendError::Closed(_) => IngestError::NotRunning,
        })
    }

    /// Register a sender under `flow_id`, replacing any prior one. Called by the
    /// source on build; the returned receiver is owned by the source.
    fn register(&self, flow_id: &str, capacity: usize) -> mpsc::Receiver<IngestDocs> {
        let (tx, rx) = mpsc::channel(capacity);
        self.senders.lock().unwrap().insert(flow_id.to_string(), tx);
        rx
    }

    /// Remove `flow_id`'s sender so further pushes see it as not running. Called
    /// when the source is dropped (flow stopped or rebuilt).
    fn unregister(&self, flow_id: &str) {
        self.senders.lock().unwrap().remove(flow_id);
    }
}

/// Constant `Retry-After` hint (seconds) returned when a flow's channel is full.
/// One second balances a prompt retry against not hammering a saturated sink; the
/// channel drains as the sink writes, so a single second is usually enough.
const RETRY_AFTER_SECS: u64 = 1;

/// Config for an `http_ingest` source. The flow id is injected by the flow
/// manager (it owns the registry key), so it is not part of the stored config.
#[derive(Debug, Clone, Deserialize, Default)]
struct HttpIngestConfig {
    /// Bounded-channel capacity in batches. Omitted → [`DEFAULT_INGEST_CAPACITY`].
    #[serde(default)]
    capacity: Option<usize>,
}

/// A push source: drains the bounded channel its flow's pushes land in, emitting
/// each push as one carrier batch for `json_to_arrow` to shape. Never ends on its
/// own — the flow's cancellation token stops it; on drop it deregisters so pushes
/// stop being accepted.
pub struct HttpIngestSource {
    flow_id: String,
    channels: IngestChannels,
    rx: mpsc::Receiver<IngestDocs>,
}

impl HttpIngestSource {
    /// Build a source for `flow_id`, registering its sender in `channels`. The
    /// capacity comes from `config.capacity` or the default. The flow manager calls
    /// this from the per-flow registry so the key matches the push route's lookup.
    pub fn build(flow_id: &str, channels: IngestChannels, config: &Value) -> EngineResult<Self> {
        let cfg: HttpIngestConfig = serde_json::from_value(config.clone())
            .map_err(|e| EngineError::Build(format!("invalid http_ingest config: {e}")))?;
        let capacity = cfg.capacity.unwrap_or(DEFAULT_INGEST_CAPACITY).max(1);
        let rx = channels.register(flow_id, capacity);
        Ok(Self {
            flow_id: flow_id.to_string(),
            channels,
            rx,
        })
    }
}

impl Drop for HttpIngestSource {
    fn drop(&mut self) {
        self.channels.unregister(&self.flow_id);
    }
}

#[async_trait::async_trait]
impl Source for HttpIngestSource {
    async fn read(&mut self) -> EngineResult<Option<RecordBatch>> {
        // `recv` is cancel-safe: a pending recv dropped on cancellation loses no
        // already-acknowledged data (pushes are at-most-once, documented on the
        // route). `None` means every sender was dropped — only the source holds
        // one besides the registry, so this is effectively end-of-process.
        match self.rx.recv().await {
            Some(docs) => Ok(Some(json_carrier_batch(&docs))),
            None => Ok(None),
        }
    }
}
