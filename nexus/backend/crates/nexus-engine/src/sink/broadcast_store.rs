//! Process-global registry mapping a run id to the broadcast channel its SSE
//! sink publishes to.
//!
//! The mirror of the collector's store, for the live path. The live runner
//! reserves a [`LiveChannel`] under a fresh id, writes the id into the sse
//! output config, and hands receivers to SSE subscribers. The channel is
//! `tokio::broadcast`: one producer (the sink), many consumers (subscribers); a
//! slow subscriber lags and is told so rather than blocking the stream.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use nexus_spi::dto::stream::StreamEvent;
use serde_json::Value;
use tokio::sync::broadcast;

/// Default fan-out buffer depth. A subscriber more than this many events behind
/// receives a `Lagged` error and resyncs rather than stalling the producer.
const CHANNEL_CAPACITY: usize = 256;

/// A live stream's broadcast channel plus its monotonic event counter. Cloning
/// shares the same underlying sender and counter.
#[derive(Clone)]
pub struct LiveChannel {
    sender: broadcast::Sender<StreamEvent>,
    seq: Arc<AtomicU64>,
}

impl LiveChannel {
    fn new() -> Self {
        let (sender, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            seq: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Publish a batch of rows as the next event. The sequence number is
    /// assigned here so it is monotonic across all subscribers.
    pub fn publish(&self, rows: Vec<Value>) {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        // Ignore the "no receivers" error: the stream stays warm for the next
        // subscriber to attach.
        let _ = self.sender.send(StreamEvent { seq, rows });
    }

    /// Open a new subscription. The receiver sees events published after this
    /// call; earlier events are not replayed (resume is `Last-Event-ID`-driven
    /// at the transport, not buffered here).
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.sender.subscribe()
    }
}

fn registry() -> &'static Mutex<HashMap<String, LiveChannel>> {
    static CHANNELS: OnceLock<Mutex<HashMap<String, LiveChannel>>> = OnceLock::new();
    CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Reserve a fresh broadcast channel for `run_id` before the live stream starts.
pub fn open(run_id: &str) -> LiveChannel {
    let channel = LiveChannel::new();
    registry()
        .lock()
        .unwrap()
        .insert(run_id.to_string(), channel.clone());
    channel
}

/// Look up the channel a running sse sink should publish to.
pub fn lookup(run_id: &str) -> Option<LiveChannel> {
    registry().lock().unwrap().get(run_id).cloned()
}

/// Remove the channel once the live stream is torn down.
pub fn close(run_id: &str) {
    registry().lock().unwrap().remove(run_id);
}
