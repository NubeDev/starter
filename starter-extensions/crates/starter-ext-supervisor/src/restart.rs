//! Restart-policy state machine (SCOPE.md R9).
//!
//! The supervisor asks `should_restart(reason)` after each child exit and
//! the tracker returns one of three answers:
//!
//! - [`RestartDecision::Restart`] — wait the next backoff, respawn.
//! - [`RestartDecision::Stop`] — the policy says "do not restart"
//!   (`RestartPolicy::Never`, or `OnCrash` with a clean exit).
//! - [`RestartDecision::Failed`] — the intensity cap is exceeded; the
//!   supervisor transitions the lifecycle to [`LifecycleState::Failed`]
//!   and stops trying.
//!
//! The intensity cap is "at most N restarts within the last M seconds";
//! its sliding window is computed from `Instant::now()` against a small
//! ring of recent restart times. M comes from the manifest's
//! `supervision.within_seconds` and is in *real* wall-clock seconds — the
//! test harness can stuff fake instants in via [`RestartTracker::record_now_for_tests`].

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use starter_ext_spi::{LifecycleState, RestartPolicy, Supervision};

/// Reason the child exited; drives the [`RestartPolicy::OnCrash`] choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// Child exited 0 (or any other "clean" code the supervisor models as
    /// non-crash, see [`RestartTracker::with_clean_exit_codes`]).
    Clean,
    /// Child crashed (signal, non-zero exit, killed for missing health
    /// pings, …).
    Crash,
}

/// The supervisor's next move after a child exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartDecision {
    /// Spawn again after the next backoff step.
    Restart,
    /// Honour the policy and let the lifecycle settle on
    /// [`LifecycleState::Stopped`]. The supervisor task exits.
    Stop,
    /// Intensity cap exceeded; transition to [`LifecycleState::Failed`].
    Failed,
}

impl RestartDecision {
    /// Map onto the lifecycle state the supervisor should publish into the
    /// registry record after this decision. `Restart` keeps the state at
    /// `Starting` (the supervisor is about to respawn).
    pub fn lifecycle(self) -> LifecycleState {
        match self {
            Self::Restart => LifecycleState::Starting,
            Self::Stop => LifecycleState::Stopped,
            Self::Failed => LifecycleState::Failed,
        }
    }
}

/// Per-extension restart tracker.
#[derive(Debug, Clone)]
pub struct RestartTracker {
    policy: RestartPolicy,
    max_restarts: u32,
    window: Duration,
    recent: VecDeque<Instant>,
    total: u64,
}

impl RestartTracker {
    /// Build from the manifest's supervision block. An extension without a
    /// `supervision:` section uses the [`Supervision::default`] shape —
    /// the supervisor only spawns process-flavour extensions and the
    /// manifest's `runtime.kind: process` is what gates entry into this
    /// crate, so defaults are fine if the operator omitted the section.
    pub fn from_manifest(sup: &Supervision) -> Self {
        Self {
            policy: sup.restart,
            max_restarts: sup.max_restarts,
            window: Duration::from_secs(sup.within_seconds.max(1) as u64),
            recent: VecDeque::new(),
            total: 0,
        }
    }

    /// Total restarts since this tracker was created. Surfaced on
    /// `GET /extensions/<id>` in Phase 2's admin routes.
    pub fn total(&self) -> u64 {
        self.total
    }

    /// Decide what to do after a child exit. Records the restart attempt
    /// into the sliding window when the decision is `Restart`.
    pub fn should_restart(&mut self, reason: ExitReason) -> RestartDecision {
        let want_restart = match (self.policy, reason) {
            (RestartPolicy::Always, _) => true,
            (RestartPolicy::OnCrash, ExitReason::Crash) => true,
            (RestartPolicy::OnCrash, ExitReason::Clean) => false,
            (RestartPolicy::Never, _) => false,
        };
        if !want_restart {
            return RestartDecision::Stop;
        }

        let now = Instant::now();
        self.prune(now);
        if self.recent.len() as u32 >= self.max_restarts {
            return RestartDecision::Failed;
        }
        self.recent.push_back(now);
        self.total = self.total.saturating_add(1);
        RestartDecision::Restart
    }

    fn prune(&mut self, now: Instant) {
        while let Some(front) = self.recent.front() {
            if now.duration_since(*front) > self.window {
                self.recent.pop_front();
            } else {
                break;
            }
        }
    }

    /// Test helper: record a restart with an explicit timestamp so the
    /// intensity-cap window can be exercised deterministically.
    #[doc(hidden)]
    pub fn record_now_for_tests(&mut self, at: Instant) {
        self.recent.push_back(at);
        self.total = self.total.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_ext_spi::{Backoff, HealthConfig, Supervision};

    fn sup(policy: RestartPolicy, max: u32, within: u32) -> Supervision {
        Supervision {
            restart: policy,
            max_restarts: max,
            within_seconds: within,
            backoff: Backoff::default(),
            health: HealthConfig::default(),
            group: None,
            shutdown_grace_ms: 5_000,
        }
    }

    #[test]
    fn never_policy_always_stops() {
        let mut t = RestartTracker::from_manifest(&sup(RestartPolicy::Never, 5, 60));
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Stop);
        assert_eq!(t.should_restart(ExitReason::Clean), RestartDecision::Stop);
    }

    #[test]
    fn on_crash_skips_clean_exit() {
        let mut t = RestartTracker::from_manifest(&sup(RestartPolicy::OnCrash, 5, 60));
        assert_eq!(t.should_restart(ExitReason::Clean), RestartDecision::Stop);
        assert_eq!(
            t.should_restart(ExitReason::Crash),
            RestartDecision::Restart
        );
    }

    #[test]
    fn intensity_cap_transitions_to_failed() {
        let mut t = RestartTracker::from_manifest(&sup(RestartPolicy::Always, 3, 60));
        // Five rapid crashes — first three restart, fourth trips the cap.
        assert_eq!(
            t.should_restart(ExitReason::Crash),
            RestartDecision::Restart
        );
        assert_eq!(
            t.should_restart(ExitReason::Crash),
            RestartDecision::Restart
        );
        assert_eq!(
            t.should_restart(ExitReason::Crash),
            RestartDecision::Restart
        );
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Failed);
        // And it stays Failed for subsequent exits.
        assert_eq!(t.should_restart(ExitReason::Crash), RestartDecision::Failed);
        assert_eq!(t.total(), 3);
    }
}
