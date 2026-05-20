//! Per-run observability counters (D-F3.10, D-F3.11).
//!
//! SCOPE: the SPI ships [`RunMetrics`] as a flat counter snapshot
//! type (`#[non_exhaustive]`, both counters `u64`). The engine
//! maintains the live counters in [`RunMetricsCell`] — two
//! lock-free `AtomicU64`s — and produces a [`RunMetrics`] snapshot
//! via [`RunMetricsCell::snapshot`] when a caller asks.
//!
//! - `subscriber_lagged_count` increments when the engine-owned
//!   `Lagged`-watcher subscriber sees a
//!   `broadcast::error::RecvError::Lagged(n)` on the per-run
//!   `FlowEvent` channel (D-F3.10).
//! - `degraded_dropped_count` increments when the per-run in-memory
//!   checkpoint queue (used while [`starter_flow_spi::flow::EngineHealth::Degraded`])
//!   exceeds its capacity and evicts the oldest batch (D-F3.11).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use starter_flow_spi::flow::{FlowEvent, RunMetrics};

/// Live per-run counters. Construct one per run; snapshot into
/// [`RunMetrics`] on demand.
#[derive(Debug, Default)]
pub struct RunMetricsCell {
    /// Cumulative `broadcast::error::RecvError::Lagged(n)` events the
    /// engine's own Lagged-watcher subscriber has seen for this
    /// run's broadcast.
    pub subscriber_lagged_count: AtomicU64,
    /// Cumulative checkpoint batches evicted from the degraded-mode
    /// queue (evict-oldest on overflow).
    pub degraded_dropped_count: AtomicU64,
}

impl RunMetricsCell {
    /// Construct a fresh, zeroed cell wrapped in an `Arc`.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Take a snapshot of the current counters into the SPI's
    /// [`RunMetrics`] shape.
    pub fn snapshot(&self) -> RunMetrics {
        let mut m = RunMetrics::zero();
        m.subscriber_lagged_count = self.subscriber_lagged_count.load(Ordering::Acquire);
        m.degraded_dropped_count = self.degraded_dropped_count.load(Ordering::Acquire);
        m
    }

    /// Add `n` to `subscriber_lagged_count`.
    pub fn add_lagged(&self, n: u64) {
        self.subscriber_lagged_count.fetch_add(n, Ordering::AcqRel);
    }

    /// Add `n` to `degraded_dropped_count`.
    pub fn add_dropped(&self, n: u64) {
        self.degraded_dropped_count.fetch_add(n, Ordering::AcqRel);
    }
}

/// Spawn the engine-owned `Lagged`-watcher subscriber for one per-run
/// broadcast (D-F3.10).
///
/// The watcher subscribes to `events_tx` and runs as a detached
/// `tokio` task that consumes events as fast as it can; on every
/// `RecvError::Lagged(n)` it adds `n` to `metrics.subscriber_lagged_count`.
/// The task exits cleanly when every other sender / receiver pair is
/// dropped and the channel closes.
///
/// The returned [`JoinHandle`] is rarely needed — the watcher is
/// fire-and-forget — but is exposed so tests can `await` task exit
/// after the channel closes.
pub fn spawn_lagged_watcher(
    events_tx: &broadcast::Sender<FlowEvent>,
    metrics: Arc<RunMetricsCell>,
) -> JoinHandle<()> {
    let mut watcher_rx = events_tx.subscribe();
    tokio::spawn(async move {
        loop {
            match watcher_rx.recv().await {
                Ok(_) => {}
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    metrics.add_lagged(n);
                }
                Err(broadcast::error::RecvError::Closed) => return,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_both_counters() {
        let c = RunMetricsCell::new();
        c.add_lagged(7);
        c.add_dropped(3);
        let s = c.snapshot();
        assert_eq!(s.subscriber_lagged_count, 7);
        assert_eq!(s.degraded_dropped_count, 3);
    }
}
