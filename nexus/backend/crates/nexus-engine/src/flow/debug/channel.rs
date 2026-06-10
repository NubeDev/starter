//! Process-global registry mapping a running flow's id to the broadcast channel
//! its per-node debug taps publish to.
//!
//! The mirror of [`crate::sink::broadcast_store`] for the flow path, with two
//! differences. First it is keyed by **flow id** (a flow is named and long-lived)
//! rather than a per-subscription run id. Second the channel carries a runtime
//! `enabled` toggle: the taps are always installed when a flow starts (so debug
//! can be turned on mid-run without rebuilding the pipeline), but every tap is
//! gated on a single relaxed atomic load — when debug is off a node pays only
//! that load and skips all sampling, row conversion, and publishing.
//!
//! The channel is `tokio::broadcast`: many producers (the source, each
//! processor, the sink) and many consumers (SSE subscribers); a slow subscriber
//! lags and is told so rather than blocking the pipeline.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use nexus_spi::dto::flow::{FlowDebugEvent, NodeCounters};
use tokio::sync::broadcast;

/// Fan-out buffer depth. A subscriber more than this many events behind receives
/// a `Lagged` error and resyncs rather than stalling the pipeline.
const CHANNEL_CAPACITY: usize = 256;

/// Default per-node cap on rows sampled per publish. Kept small so a high-volume
/// flow streams a representative slice, not the whole batch.
pub const DEFAULT_SAMPLE_ROWS: usize = 20;

/// A running flow's debug broadcast channel plus its monotonic event counter and
/// runtime enable toggle. Cloning shares the same sender, counter, and toggle.
#[derive(Clone)]
pub struct FlowDebugChannel {
    sender: broadcast::Sender<FlowDebugEvent>,
    seq: Arc<AtomicU64>,
    enabled: Arc<AtomicBool>,
    /// Latest counters published per node, keyed by `node_index`. A
    /// `broadcast` subscriber sees only events published *after* it attaches, so
    /// without this a node that hasn't had a batch cross since the SSE stream
    /// opened would stay blank ("—") until its next batch — and on a bursty
    /// source that can be seconds. We snapshot the running totals here so a
    /// freshly-attached subscriber can be replayed the current state up front
    /// (see [`Self::snapshot`]). Totals are monotonic, so the replay is always
    /// truthful; no staleness, no heartbeat needed.
    latest: Arc<Mutex<HashMap<u32, NodeCounters>>>,
    /// Total nodes (source + processors + sink), so the API can report the chain
    /// length and the UI can validate its positional mapping.
    node_count: u32,
    /// Per-node cap on rows sampled per publish.
    sample_rows: usize,
}

impl FlowDebugChannel {
    fn new(node_count: u32) -> Self {
        let (sender, _rx) = broadcast::channel(CHANNEL_CAPACITY);
        Self {
            sender,
            seq: Arc::new(AtomicU64::new(0)),
            enabled: Arc::new(AtomicBool::new(false)),
            latest: Arc::new(Mutex::new(HashMap::new())),
            node_count,
            sample_rows: DEFAULT_SAMPLE_ROWS,
        }
    }

    /// Whether sampling/publishing is currently on. Checked by every tap at the
    /// top of its hot path — a single relaxed load when debug is off.
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Turn value/sample capture on or off for the running flow.
    pub fn set_enabled(&self, on: bool) {
        self.enabled.store(on, Ordering::Relaxed);
    }

    /// The flow's node count (source + processors + sink).
    pub fn node_count(&self) -> u32 {
        self.node_count
    }

    /// The per-node sample-row cap.
    pub fn sample_rows(&self) -> usize {
        self.sample_rows
    }

    /// The next monotonic sequence number, assigned to an event before publish.
    pub fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::Relaxed)
    }

    /// Publish one event. The caller has already gated on [`is_enabled`] for the
    /// hot-path variants. The "no receivers" error is ignored: the channel stays
    /// warm for the next subscriber to attach.
    ///
    /// A `Counters` event also updates the per-node snapshot so a subscriber that
    /// attaches later can be brought up to the current totals immediately rather
    /// than waiting for the node's next batch (see [`Self::snapshot`]).
    pub fn publish(&self, event: FlowDebugEvent) {
        if let FlowDebugEvent::Counters { counters, .. } = &event {
            self.latest
                .lock()
                .unwrap()
                .insert(counters.node_index, counters.clone());
        }
        let _ = self.sender.send(event);
    }

    /// Open a new subscription. The receiver sees events published after this
    /// call; earlier events are not replayed — pair with [`Self::snapshot`] to
    /// prime the new subscriber with the current per-node totals.
    pub fn subscribe(&self) -> broadcast::Receiver<FlowDebugEvent> {
        self.sender.subscribe()
    }

    /// The latest per-node counters seen so far, ordered by `node_index`. Replayed
    /// to a newly-attached SSE subscriber so the panel shows the true running
    /// totals on connect instead of blanks until the next batch crosses each node.
    pub fn snapshot(&self) -> Vec<NodeCounters> {
        let mut rows: Vec<NodeCounters> = self.latest.lock().unwrap().values().cloned().collect();
        rows.sort_by_key(|c| c.node_index);
        rows
    }
}

fn registry() -> &'static Mutex<HashMap<String, FlowDebugChannel>> {
    static CHANNELS: OnceLock<Mutex<HashMap<String, FlowDebugChannel>>> = OnceLock::new();
    CHANNELS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Reserve a fresh debug channel for `flow_id` before its run starts, replacing
/// any stale channel from a previous run. Debug defaults to off.
pub fn open(flow_id: &str, node_count: u32) -> FlowDebugChannel {
    let channel = FlowDebugChannel::new(node_count);
    registry()
        .lock()
        .unwrap()
        .insert(flow_id.to_string(), channel.clone());
    channel
}

/// Look up a running flow's debug channel — used by the SSE stream and the
/// enable/disable endpoints. `None` if the flow is not running on this node.
pub fn lookup(flow_id: &str) -> Option<FlowDebugChannel> {
    registry().lock().unwrap().get(flow_id).cloned()
}

/// Remove the channel once the flow's run ends, on every terminal path.
pub fn close(flow_id: &str) {
    registry().lock().unwrap().remove(flow_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_spi::dto::flow::{LogLevel, NodeRole};

    fn counters_event(seq: u64) -> FlowDebugEvent {
        FlowDebugEvent::Counters {
            seq,
            counters: nexus_spi::dto::flow::NodeCounters {
                node_index: 0,
                role: NodeRole::Source,
                rows_in: 1,
                rows_out: 1,
                batches: 1,
            },
        }
    }

    #[test]
    fn defaults_to_disabled() {
        let channel = FlowDebugChannel::new(3);
        assert!(!channel.is_enabled());
        assert_eq!(channel.node_count(), 3);
    }

    #[test]
    fn toggle_flips_enabled() {
        let channel = FlowDebugChannel::new(2);
        channel.set_enabled(true);
        assert!(channel.is_enabled());
        channel.set_enabled(false);
        assert!(!channel.is_enabled());
    }

    #[tokio::test]
    async fn subscriber_receives_published_events_in_seq_order() {
        let channel = FlowDebugChannel::new(2);
        let mut rx = channel.subscribe();
        channel.publish(counters_event(channel.next_seq()));
        channel.publish(FlowDebugEvent::Log {
            seq: channel.next_seq(),
            level: LogLevel::Warn,
            node_index: Some(1),
            message: "retry".into(),
            at_ms: 0,
        });

        match rx.recv().await.unwrap() {
            FlowDebugEvent::Counters { seq, .. } => assert_eq!(seq, 0),
            other => panic!("expected counters, got {other:?}"),
        }
        match rx.recv().await.unwrap() {
            FlowDebugEvent::Log { seq, message, .. } => {
                assert_eq!(seq, 1);
                assert_eq!(message, "retry");
            }
            other => panic!("expected log, got {other:?}"),
        }
    }

    #[test]
    fn snapshot_returns_latest_counters_per_node_sorted() {
        let channel = FlowDebugChannel::new(3);
        // Two ticks for the source; the snapshot must keep the latest, not both.
        channel.publish(counters_event(channel.next_seq())); // node 0, rows_in 1
        let mut later = match counters_event(channel.next_seq()) {
            FlowDebugEvent::Counters { counters, .. } => counters,
            _ => unreachable!(),
        };
        later.rows_in = 42;
        channel.publish(FlowDebugEvent::Counters {
            seq: channel.next_seq(),
            counters: later,
        });
        // A sink at a higher index, published first — snapshot must sort by index.
        channel.publish(FlowDebugEvent::Counters {
            seq: channel.next_seq(),
            counters: NodeCounters {
                node_index: 2,
                role: NodeRole::Sink,
                rows_in: 7,
                rows_out: 7,
                batches: 1,
            },
        });

        let snap = channel.snapshot();
        assert_eq!(snap.len(), 2, "one entry per node, deduped");
        assert_eq!(snap[0].node_index, 0);
        assert_eq!(snap[0].rows_in, 42, "kept the latest tick");
        assert_eq!(snap[1].node_index, 2);
    }

    #[test]
    fn snapshot_ignores_non_counter_events() {
        let channel = FlowDebugChannel::new(2);
        channel.publish(FlowDebugEvent::Log {
            seq: channel.next_seq(),
            level: LogLevel::Info,
            node_index: Some(0),
            message: "hi".into(),
            at_ms: 0,
        });
        assert!(channel.snapshot().is_empty());
    }

    #[test]
    fn registry_open_lookup_close_round_trip() {
        let id = "test-flow-debug-roundtrip";
        assert!(lookup(id).is_none());
        let channel = open(id, 4);
        assert_eq!(channel.node_count(), 4);
        let found = lookup(id).expect("registered");
        // The looked-up clone shares the toggle.
        found.set_enabled(true);
        assert!(channel.is_enabled());
        close(id);
        assert!(lookup(id).is_none());
    }
}
