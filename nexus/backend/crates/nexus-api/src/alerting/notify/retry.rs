//! Bounded retry-with-backoff for channel delivery.
//!
//! A flaky downstream (a Slack rate-limit, an SMTP hiccup) should not lose a
//! notification on the first failure, but alerting must not block the evaluator
//! indefinitely either. So delivery is retried a small fixed number of times with
//! exponential backoff; the durable queue the design defers is a later hardening.
//! The attempt count and last error are returned so the evaluator records them on
//! the event — the operator can see "delivered after 2 retries" or "failed ×3".

use std::future::Future;
use std::time::Duration;

/// The default delivery attempt budget (1 initial + this many retries).
pub const MAX_RETRIES: u32 = 3;

/// The base backoff; attempt N waits `BASE * 2^(N-1)` before retrying.
const BASE_BACKOFF: Duration = Duration::from_millis(200);

/// The outcome of a retried delivery: how many attempts ran and the last error
/// if every attempt failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryOutcome {
    pub attempts: u32,
    pub last_error: Option<String>,
}

impl DeliveryOutcome {
    /// Whether delivery ultimately succeeded.
    pub fn succeeded(&self) -> bool {
        self.last_error.is_none()
    }
}

/// Run `attempt` up to `1 + max_retries` times with exponential backoff between
/// tries, stopping on the first success. `sleep` is injected so tests drive the
/// backoff without real time; production passes `tokio::time::sleep`.
pub async fn with_backoff<F, Fut, S, SFut>(
    max_retries: u32,
    mut attempt: F,
    mut sleep: S,
) -> DeliveryOutcome
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<(), String>>,
    S: FnMut(Duration) -> SFut,
    SFut: Future<Output = ()>,
{
    let mut attempts = 0;
    let mut last_error = None;
    for n in 0..=max_retries {
        attempts += 1;
        match attempt().await {
            Ok(()) => {
                return DeliveryOutcome {
                    attempts,
                    last_error: None,
                }
            }
            Err(e) => last_error = Some(e),
        }
        if n < max_retries {
            sleep(backoff_for(n)).await;
        }
    }
    DeliveryOutcome {
        attempts,
        last_error,
    }
}

/// The backoff before the retry following attempt index `n` (0-based).
fn backoff_for(n: u32) -> Duration {
    BASE_BACKOFF * 2u32.pow(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    async fn no_sleep(_d: Duration) {}

    #[tokio::test]
    async fn succeeds_first_try_runs_once() {
        let out = with_backoff(3, || async { Ok(()) }, no_sleep).await;
        assert_eq!(out.attempts, 1);
        assert!(out.succeeded());
    }

    #[tokio::test]
    async fn retries_then_succeeds() {
        let calls = Cell::new(0u32);
        let out = with_backoff(
            3,
            || async {
                let n = calls.get();
                calls.set(n + 1);
                if n < 2 {
                    Err("transient".to_string())
                } else {
                    Ok(())
                }
            },
            no_sleep,
        )
        .await;
        assert_eq!(out.attempts, 3);
        assert!(out.succeeded());
    }

    #[tokio::test]
    async fn exhausts_retries_and_reports_last_error() {
        let out = with_backoff(2, || async { Err("down".to_string()) }, no_sleep).await;
        assert_eq!(out.attempts, 3); // 1 initial + 2 retries
        assert!(!out.succeeded());
        assert_eq!(out.last_error.as_deref(), Some("down"));
    }

    #[test]
    fn backoff_grows_exponentially() {
        assert_eq!(backoff_for(0), BASE_BACKOFF);
        assert_eq!(backoff_for(1), BASE_BACKOFF * 2);
        assert_eq!(backoff_for(2), BASE_BACKOFF * 4);
    }
}
