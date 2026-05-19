//! [`RetryPolicy`] — exponential-backoff retry layer with a max-attempts
//! circuit, per SCOPE R9.
//!
//! The registry does **not** auto-restart a failed service (R9). If the
//! service wants to recover from a transport blip it owns that policy
//! itself; this module is that policy.
//!
//! Two behaviours coexist:
//!
//! 1. **Reset on success.** A clean disconnect (the websocket closed
//!    after the bot had been pumping events) is *not* a failure for
//!    backoff purposes — Slack rotates connections every ~30 minutes.
//!    The consecutive-failure counter resets to zero whenever
//!    `record_success` is called.
//!
//! 2. **Trip after `max_attempts`.** A persistent failure mode
//!    (`apps.connections.open` rejects the token, the WSS URL is
//!    permanently malformed, …) would otherwise pin the service in a
//!    hot reconnect loop. After `max_attempts` *consecutive* failures
//!    [`RetryPolicy::next_step`] returns [`RetryStep::Trip`] and the
//!    service exits.
//!
//! The defaults are deliberately small for tests: production callers
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
    /// Sleep `backoff`, then retry the connect.
    Backoff {
        /// How long the caller should sleep (with the shutdown signal
        /// race) before the next attempt.
        backoff: Duration,
        /// Attempt number that just failed (1-based).
        attempt: u32,
    },
    /// Stop retrying; the circuit is open. Caller should exit the
    /// outer loop.
    Trip {
        /// Total number of consecutive failures that triggered the
        /// trip. Useful for the operator log line and the
        /// [`crate::SlackSocketModeError::CircuitTripped`] payload.
        attempts: u32,
    },
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_secs(1),
            max_backoff: Duration::from_secs(60),
            // Six consecutive failures with the default doubling
            // backoff (1, 2, 4, 8, 16, 32 ≈ 63s) gives Slack a full
            // minute to come back before the circuit trips.
            max_attempts: 6,
            consecutive_failures: 0,
        }
    }
}

impl RetryPolicy {
    /// Builder-style override for the initial backoff. The default is
    /// `1s`.
    pub fn with_initial_backoff(mut self, d: Duration) -> Self {
        self.initial_backoff = d;
        self
    }

    /// Builder-style override for the backoff cap. The default is
    /// `60s`.
    pub fn with_max_backoff(mut self, d: Duration) -> Self {
        self.max_backoff = d;
        self
    }

    /// Builder-style override for the circuit trip threshold. The
    /// default is `6` consecutive failures.
    pub fn with_max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    /// Reset the consecutive-failure counter. Call after every clean
    /// disconnect — a 30-minute uptime followed by Slack rotating the
    /// socket is normal and must not count toward the circuit.
    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Record one failure and ask the policy what to do next.
    pub fn next_step(&mut self) -> RetryStep {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= self.max_attempts {
            return RetryStep::Trip {
                attempts: self.consecutive_failures,
            };
        }
        // Doubling backoff from the initial, capped at the maximum.
        // `consecutive_failures - 1` so the first failure uses
        // `initial_backoff`, not `initial_backoff * 2`.
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
        // After reset we should still be two failures away from trip.
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        assert!(matches!(p.next_step(), RetryStep::Backoff { .. }));
        assert!(matches!(p.next_step(), RetryStep::Trip { .. }));
    }
}
