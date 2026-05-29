//! `starter-ext-metrics` — the per-extension counter registry.
//!
//! This is a **leaf crate** (locked decision §3 in
//! `comprehensive-extension-management.md`). It holds nothing but a
//! `DashMap<ExtensionId, Counters>` of atomic tallies. The transport
//! adapters — `starter-ext-mcp` (tool calls/errors), `starter-ext-server`
//! REST dispatch (rest requests), and `starter-ext-workers` (worker
//! runs/failures) — take a cheap `&MetricsRegistry` handle at wiring time
//! and bump a counter on their hot path. The supervisor reads the same
//! registry and folds in the process gauges to build the merged
//! [`ExtensionMetrics`] served by `GET /extensions/<id>/metrics`.
//!
//! Keeping the registry here means every dependency arrow points one way:
//! adapters → metrics ← supervisor. No adapter has to depend on the
//! supervisor (process spawning, signals, `/proc` sampling) just to record
//! that a tool was called, and the supervisor does not have to know about
//! any transport.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use dashmap::DashMap;
use starter_ext_spi::{ExtensionId, ExtensionMetrics, LifecycleState, ProcessStats};

/// The atomic counters one extension accumulates over the host's lifetime.
/// Every field is monotone. Cloned out (not moved) when building the merged
/// view, so the live tallies keep counting.
#[derive(Debug, Default)]
pub struct Counters {
    tool_calls: AtomicU64,
    tool_errors: AtomicU64,
    rest_requests: AtomicU64,
    worker_runs: AtomicU64,
    worker_failures: AtomicU64,
}

impl Counters {
    /// Record a tool invocation dispatched through `starter-ext-mcp`.
    #[inline]
    pub fn record_tool_call(&self) {
        self.tool_calls.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a tool invocation that ended in an error. Callers bump this
    /// *in addition to* [`Self::record_tool_call`], so `tool_errors` is a
    /// subset of `tool_calls`.
    #[inline]
    pub fn record_tool_error(&self) {
        self.tool_errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a REST request dispatched through `starter-ext-server`.
    #[inline]
    pub fn record_rest_request(&self) {
        self.rest_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a periodic-worker run started by `starter-ext-workers`.
    #[inline]
    pub fn record_worker_run(&self) {
        self.worker_runs.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a periodic-worker run that failed. Bumped *in addition to*
    /// [`Self::record_worker_run`], so `worker_failures` is a subset of
    /// `worker_runs`.
    #[inline]
    pub fn record_worker_failure(&self) {
        self.worker_failures.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            tool_calls: self.tool_calls.load(Ordering::Relaxed),
            tool_errors: self.tool_errors.load(Ordering::Relaxed),
            rest_requests: self.rest_requests.load(Ordering::Relaxed),
            worker_runs: self.worker_runs.load(Ordering::Relaxed),
            worker_failures: self.worker_failures.load(Ordering::Relaxed),
        }
    }
}

/// A point-in-time read of one extension's [`Counters`]. Used internally to
/// build the merged view; also exposed for tests / debugging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CounterSnapshot {
    pub tool_calls: u64,
    pub tool_errors: u64,
    pub rest_requests: u64,
    pub worker_runs: u64,
    pub worker_failures: u64,
}

/// The process-side gauges the supervisor supplies when merging. Kept as a
/// plain value type so this crate never has to depend on the supervisor —
/// the caller fills it from its `SupervisorHandle`.
#[derive(Debug, Clone, PartialEq)]
pub struct ProcessGauges {
    /// Sampled process stats; `None` for builtin/wasm or when not running.
    pub process: Option<ProcessStats>,
    /// Current lifecycle state.
    pub lifecycle_state: LifecycleState,
    /// Cumulative restarts.
    pub restarts_total: u64,
    /// Cumulative capability violations.
    pub capability_violations_total: u64,
    /// Event-ring evictions.
    pub events_dropped_total: u64,
}

/// A cheap-to-clone handle to the per-extension counter map. Adapters keep
/// a clone and call [`Self::counters`] on their hot path; the supervisor
/// keeps a clone and calls [`Self::merged`] to build the response.
#[derive(Debug, Clone, Default)]
pub struct MetricsRegistry {
    inner: Arc<DashMap<ExtensionId, Arc<Counters>>>,
}

impl MetricsRegistry {
    /// A fresh, empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get (creating if absent) the [`Counters`] for an extension. The
    /// returned `Arc` can be held by the adapter so subsequent bumps avoid
    /// the map lookup entirely.
    pub fn counters(&self, id: &ExtensionId) -> Arc<Counters> {
        if let Some(c) = self.inner.get(id) {
            return Arc::clone(c.value());
        }
        Arc::clone(
            self.inner
                .entry(id.clone())
                .or_insert_with(|| Arc::new(Counters::default()))
                .value(),
        )
    }

    /// Snapshot one extension's counters, or all-zero when the extension
    /// has never been recorded against.
    pub fn snapshot(&self, id: &ExtensionId) -> CounterSnapshot {
        self.inner
            .get(id)
            .map(|c| c.value().snapshot())
            .unwrap_or_default()
    }

    /// Fold the adapter counters with the supervisor-supplied process
    /// gauges into the merged [`ExtensionMetrics`] view. This is the single
    /// projection point; the `GET /extensions/<id>/metrics` handler calls
    /// it.
    pub fn merged(&self, id: &ExtensionId, gauges: ProcessGauges) -> ExtensionMetrics {
        let c = self.snapshot(id);
        ExtensionMetrics {
            process: gauges.process,
            lifecycle_state: gauges.lifecycle_state,
            restarts_total: gauges.restarts_total,
            capability_violations_total: gauges.capability_violations_total,
            tool_calls_total: c.tool_calls,
            tool_errors_total: c.tool_errors,
            rest_requests_total: c.rest_requests,
            worker_runs_total: c.worker_runs,
            worker_failures_total: c.worker_failures,
            events_dropped_total: gauges.events_dropped_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> ExtensionId {
        ExtensionId::new("com.acme.demo").unwrap()
    }

    #[test]
    fn counters_increment_independently() {
        let reg = MetricsRegistry::new();
        let c = reg.counters(&id());
        c.record_tool_call();
        c.record_tool_call();
        c.record_tool_error();
        c.record_rest_request();
        c.record_worker_run();
        c.record_worker_run();
        c.record_worker_failure();

        let s = reg.snapshot(&id());
        assert_eq!(s.tool_calls, 2);
        assert_eq!(s.tool_errors, 1);
        assert_eq!(s.rest_requests, 1);
        assert_eq!(s.worker_runs, 2);
        assert_eq!(s.worker_failures, 1);
    }

    #[test]
    fn counters_handle_is_shared_not_copied() {
        let reg = MetricsRegistry::new();
        // Two independent lookups must land on the same atomics.
        reg.counters(&id()).record_tool_call();
        reg.counters(&id()).record_tool_call();
        assert_eq!(reg.snapshot(&id()).tool_calls, 2);
    }

    #[test]
    fn unknown_extension_snapshots_to_zero() {
        let reg = MetricsRegistry::new();
        assert_eq!(reg.snapshot(&id()), CounterSnapshot::default());
    }

    #[test]
    fn merged_projection_combines_both_sources() {
        let reg = MetricsRegistry::new();
        let c = reg.counters(&id());
        c.record_tool_call();
        c.record_tool_call();
        c.record_tool_error();
        c.record_rest_request();
        c.record_worker_run();
        c.record_worker_failure();

        let merged = reg.merged(
            &id(),
            ProcessGauges {
                process: None,
                lifecycle_state: LifecycleState::Running,
                restarts_total: 3,
                capability_violations_total: 2,
                events_dropped_total: 7,
            },
        );

        // Adapter counters.
        assert_eq!(merged.tool_calls_total, 2);
        assert_eq!(merged.tool_errors_total, 1);
        assert_eq!(merged.rest_requests_total, 1);
        assert_eq!(merged.worker_runs_total, 1);
        assert_eq!(merged.worker_failures_total, 1);
        // Supervisor gauges.
        assert_eq!(merged.lifecycle_state, LifecycleState::Running);
        assert_eq!(merged.restarts_total, 3);
        assert_eq!(merged.capability_violations_total, 2);
        assert_eq!(merged.events_dropped_total, 7);
        assert!(merged.process.is_none());
    }
}
