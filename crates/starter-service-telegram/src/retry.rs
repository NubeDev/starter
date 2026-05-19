//! [`RetryPolicy`] — exponential-backoff retry layer with a
//! max-attempts circuit, per SCOPE R9.
//!
//! Mirrors `starter-service-slack::retry` so the two services have
//! the same operator-visible behaviour. The differences are
//! semantic, not structural:
//!
//! 1. **Reset on success.** A successful long-poll resets the
//!    consecutive-failure counter — the loop normally cycles forever
//!    on success and only sleeps on errors, so a recovered transient
//!    failure must not count toward the circuit.
//!
//! 2. **Trip after `max_attempts`.** A persistent failure mode pins
//!    the loop in a hot reconnect otherwise. After `max_attempts`
//!    consecutive failures [`RetryPolicy::next_step`] returns
//!    [`RetryStep::Trip`] and the service exits.
//!
//! Defaults are deliberately small for tests; production callers
//! should tune via [`RetryPolicy::with_*`].

use std::time::Duration;

/// Exponential backoff + circuit breaker.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    initial_backoff: Duration,
    max_backoff: Duration,
    max_attempts: u32,
    consecutive_failures: u32,
}

/// Decision returned by [`RetryPolicy::next_step`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStep {
    /// Sleep `backoff`, then retry.
    Backoff {
        /// How long the caller should sleep (with the shutdown
        /// signal race) before the next attempt.
        backoff: Duration,
        /// Attempt number that just failed (1-based).
        attempt: u32,
    },
    /// Stop retrying; the circuit is open.
    Trip {
        /// Total number of consecutive failures that triggered the
        /// trip.
        attempts: u32,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            // Six consecutive failures with the default doubling
            // backoff (1, 2, 4, 8, 16, 32 ≈ 63s) gives Telegram a
            // full minute to come back before the circuit trips.
            max_attempts: 6,
            consecutive_failures: 0,
        }
    }
}

impl RetryPolicy {
    /// Builder-style override for the initial backoff (default `1s`).
    pub fn with_initial_backoff(mut self, d: Duration) -> Self {
        self.initial_backoff = d;
        self
    }

    /// Builder-style override for the backoff cap (default `60s`).
    pub fn with_max_backoff(mut self, d: Duration) -> Self {
        self.max_backoff = d;
        self
    }

    /// Builder-style override for the circuit trip threshold
    /// (default `6` consecutive failures).
    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    /// Reset the consecutive-failure counter. Call after every
    /// successful poll.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Trip the circuit immediately on a non-transient error (401 /
    /// 404). Bypasses the backoff schedule so the operator sees the
    /// cause on the first failure.
    pub(crate) fn trip_immediately(&mut self) -> u32 {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.consecutive_failures
    }

    /// Record one failure and ask the policy what to do next.
    pub fn next_step(&mut self) -> RetryStep {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.max_attempts {
            return RetryStep::Trip {
                attempts: self.consecutive_failures,
            };
        }
        let shift = (self.consecutive_failures - 1).min(30);
        let factor: u64 = 1u64.checked_shl(shift).unwrap_or(u64::MAX);
        let candidate = self
            .initial_backoff
            .checked_mul(factor.min(u32::MAX as u64) as u32)
            .unwrap_or(self.max_backoff);
        let backoff = candidate.min(self.max_backoff);
        RetryStep::Backoff {
            backoff,
            attempt: self.consecutive_failures,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_then_caps() {
        let mut p = RetryPolicy::default()
            .with_initial_backoff(Duration::from_secs(1))
            .with_max_backoff(Duration::from_secs(8))
            .with_max_attempts(10);
        let mut steps = Vec::new();
        for _ in 0..6 {
            match p.next_step() {
                RetryStep::Backoff { backoff, .. } => steps.push(backoff),
                RetryStep::Trip { .. } => panic!("did not expect trip"),
            }
        }
        assert_eq!(
            steps,
            vec![
                Duration::from_secs(1),
                Duration::from_secs(2),
                Duration::from_secs(4),
                Duration::from_secs(8),
                Duration::from_secs(8),
                Duration::from_secs(8),
            ]
        );
    }

    #[test]
    fn trips_after_max_attempts() {
        let mut p = RetryPolicy::default().with_max_attempts(3);
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        match p.next_step() {
            RetryStep::Trip { attempts } => assert_eq!(attempts, 3),
            other => panic!("expected Trip, got {other:?}"),
        }
    }

    #[test]
    fn record_success_resets_the_counter() {
        let mut p = RetryPolicy::default().with_max_attempts(3);
        let _ = p.next_step();
        let _ = p.next_step();
        p.record_success();
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        assert!(matches!(p.next_step(), RetryStep::Trip { .. }));
    }
}
