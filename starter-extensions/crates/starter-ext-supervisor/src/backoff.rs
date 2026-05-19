//! Exponential backoff schedule with optional jitter.
//!
//! Per SCOPE.md **R9**: restart wait is exponential with jitter, capped at
//! the manifest's `backoff.max_ms`. The intensity cap (max N restarts
//! within M seconds) is a *separate* concern handled by
//! [`crate::restart::RestartTracker`] — this module only computes the next
//! sleep duration.
//!
//! Pulling the schedule out of the supervisor's hot loop keeps the loop
//! itself testable without a runtime: the supervisor produces a
//! [`BackoffSchedule`], asks for the next wait, then chooses how to
//! `tokio::time::sleep` against it.

use std::time::Duration;

use starter_ext_spi::Backoff;

/// Iterator-style schedule that returns the next sleep duration. Doubles
/// each step until [`Backoff::max_ms`] is reached; optionally adds a small
/// random jitter to spread restarts across replicas.
///
/// The schedule resets to `initial_ms` after a successful start (a child
/// that ran past the health-check window is "stable" and the next crash
/// should not pay for previous attempts). The supervisor owns the reset
/// call site.
#[derive(Debug, Clone)]
pub struct BackoffSchedule {
    initial: Duration,
    max: Duration,
    jitter: bool,
    next: Duration,
}

impl BackoffSchedule {
    /// Build a schedule from the manifest's [`Backoff`] block.
    pub fn from_manifest(cfg: &Backoff) -> Self {
        let initial = Duration::from_millis(cfg.initial_ms.max(1) as u64);
        let max = Duration::from_millis(cfg.max_ms.max(cfg.initial_ms) as u64);
        Self {
            initial,
            max,
            jitter: cfg.jitter,
            next: initial,
        }
    }

    /// Reset to the initial wait. Called after a child has been "stable"
    /// long enough that the supervisor considers prior crashes paid for.
    pub fn reset(&mut self) {
        self.next = self.initial;
    }

    /// Take the next wait. Updates the internal counter — calling
    /// repeatedly walks the schedule.
    pub fn next_wait(&mut self) -> Duration {
        let base = self.next.min(self.max);
        // Compute the *following* step's base before applying jitter so
        // the schedule shape is deterministic given the same seed.
        let following = base.saturating_mul(2).min(self.max);
        self.next = if following == Duration::ZERO {
            self.initial
        } else {
            following
        };

        if self.jitter {
            // Add up to 50% jitter on top of `base`. 50% spread is large
            // enough to dispersion a thundering herd, small enough that
            // the next backoff still arrives in a predictable window.
            let extra_ms = (base.as_millis() as u64).saturating_div(2);
            if extra_ms > 0 {
                let jitter_ms = rand::random::<u64>() % (extra_ms + 1);
                return base + Duration::from_millis(jitter_ms);
            }
        }
        base
    }

    /// Peek the next wait without consuming it. Useful for diagnostic
    /// surfaces that want to display "next restart in ~5s".
    pub fn peek(&self) -> Duration {
        self.next.min(self.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(initial: u32, max: u32, jitter: bool) -> Backoff {
        Backoff {
            initial_ms: initial,
            max_ms: max,
            jitter,
        }
    }

    #[test]
    fn doubles_until_cap() {
        let mut s = BackoffSchedule::from_manifest(&cfg(100, 1_000, false));
        assert_eq!(s.next_wait(), Duration::from_millis(100));
        assert_eq!(s.next_wait(), Duration::from_millis(200));
        assert_eq!(s.next_wait(), Duration::from_millis(400));
        assert_eq!(s.next_wait(), Duration::from_millis(800));
        // Capped here.
        assert_eq!(s.next_wait(), Duration::from_millis(1_000));
        assert_eq!(s.next_wait(), Duration::from_millis(1_000));
    }

    #[test]
    fn reset_returns_to_initial() {
        let mut s = BackoffSchedule::from_manifest(&cfg(50, 500, false));
        let _ = s.next_wait();
        let _ = s.next_wait();
        s.reset();
        assert_eq!(s.next_wait(), Duration::from_millis(50));
    }

    #[test]
    fn jitter_stays_within_bounds() {
        let mut s = BackoffSchedule::from_manifest(&cfg(100, 100, true));
        // With cap == initial, base is 100ms; jitter adds 0..=50ms.
        for _ in 0..20 {
            let w = s.next_wait();
            assert!(w >= Duration::from_millis(100));
            assert!(w <= Duration::from_millis(150));
        }
    }
}
