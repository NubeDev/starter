//! [`ExtensionMetrics`] — the merged "how is it doing?" view served by
//! `GET /extensions/<id>/metrics`.
//!
//! Metrics are **sampled, not pushed** (plan rule 4). The aggregate folds
//! two sources that both already exist:
//!
//! - **Process gauges** from `starter-ext-supervisor`: the live
//!   [`ProcessStats`] (process-flavour only — `None` for builtin/wasm or
//!   when not `Running`), the current [`LifecycleState`], the cumulative
//!   restart count, the capability-violation counter, and the ring's
//!   eviction count (`events_dropped_total`).
//! - **Adapter counters** from the leaf crate `starter-ext-metrics`: the
//!   monotone tool / REST / worker tallies the transport adapters bump on
//!   their hot paths.
//!
//! This struct is the wire contract only; the merge logic lives in
//! `starter-ext-metrics` (`MetricsRegistry::merged`) so the dependency
//! arrows stay one-way (adapters → metrics ← supervisor) and nothing here
//! pulls in the supervisor.
//!
//! [`LifecycleState`]: crate::lifecycle::LifecycleState
//! [`ProcessStats`]: crate::process::ProcessStats

use serde::{Deserialize, Serialize};

use crate::lifecycle::LifecycleState;
use crate::process::ProcessStats;

/// The merged metrics view for a single extension. Every counter is
/// monotone and cumulative since the host started; gauges (`process`,
/// `lifecycle_state`) reflect the latest sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionMetrics {
    /// Sampled process stats for the current child. `None` for
    /// builtin/wasm flavours and for a process-flavour extension that is
    /// not currently `Running`. Reuses §2's [`ProcessStats`].
    pub process: Option<ProcessStats>,
    /// Current lifecycle state of the extension.
    pub lifecycle_state: LifecycleState,
    /// Cumulative restarts the supervisor has performed.
    pub restarts_total: u64,
    /// Cumulative capability violations the host has refused.
    pub capability_violations_total: u64,
    /// Tool invocations dispatched through `starter-ext-mcp`.
    pub tool_calls_total: u64,
    /// Tool invocations that ended in an error (subset of
    /// `tool_calls_total`).
    pub tool_errors_total: u64,
    /// REST requests dispatched through `starter-ext-server`.
    pub rest_requests_total: u64,
    /// Periodic-worker runs started by `starter-ext-workers`.
    pub worker_runs_total: u64,
    /// Periodic-worker runs that failed (subset of `worker_runs_total`).
    pub worker_failures_total: u64,
    /// Event-ring evictions — entries pushed out of the bounded ring
    /// (monotone, derived from the ring's sequence cursor vs its length).
    pub events_dropped_total: u64,
    /// Process-group `SIGKILL` escalations — times the supervisor had to
    /// reap this extension's whole process group because the child (or a
    /// grandchild it forked) did not exit on its own. `0` for a
    /// well-behaved extension; a steadily rising value flags one that leaks
    /// descendants or ignores `SIGTERM`. Always `0` on non-unix.
    #[serde(default)]
    pub group_kills_total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let m = ExtensionMetrics {
            process: None,
            lifecycle_state: LifecycleState::Running,
            restarts_total: 2,
            capability_violations_total: 1,
            tool_calls_total: 10,
            tool_errors_total: 3,
            rest_requests_total: 42,
            worker_runs_total: 7,
            worker_failures_total: 1,
            events_dropped_total: 5,
            group_kills_total: 2,
        };
        let j = serde_json::to_value(&m).unwrap();
        assert_eq!(j["lifecycle_state"], "running");
        assert_eq!(j["tool_calls_total"], 10);
        assert!(j["process"].is_null());
        let back: ExtensionMetrics = serde_json::from_value(j).unwrap();
        assert_eq!(back, m);
    }
}
