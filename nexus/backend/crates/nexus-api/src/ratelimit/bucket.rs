//! The token-bucket state machine, one bucket per tenant.
//!
//! A bucket holds up to `burst` tokens and refills at `refill_per_sec`. Each
//! admitted request spends one token; refill is computed lazily from elapsed
//! time on each check, so no background timer is needed. When the bucket is
//! empty the request is denied and the caller is told how long until the next
//! token (`Retry-After`).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Rate-limit configuration. The default (per the spec's "sensible defaults")
/// allows a steady stream of dashboard refreshes plus a generous burst.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Tokens added per second (the sustained request rate per tenant).
    pub refill_per_sec: f64,
    /// Bucket ceiling — the largest burst a tenant can spend at once.
    pub burst: f64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            refill_per_sec: 50.0,
            burst: 200.0,
        }
    }
}

impl RateLimitConfig {
    /// Read `NEXUS_TENANT_RATE_PER_SEC` and `NEXUS_TENANT_RATE_BURST` from the
    /// environment, keeping defaults for absent/unparseable/non-positive values.
    pub fn from_env() -> Self {
        let default = Self::default();
        let refill_per_sec = env_f64("NEXUS_TENANT_RATE_PER_SEC")
            .filter(|v| *v > 0.0)
            .unwrap_or(default.refill_per_sec);
        let burst = env_f64("NEXUS_TENANT_RATE_BURST")
            .filter(|v| *v > 0.0)
            .unwrap_or(default.burst);
        Self {
            refill_per_sec,
            burst,
        }
    }
}

/// One tenant's bucket: current token count and when it was last refilled.
struct Bucket {
    tokens: f64,
    last_refill: Instant,
}

/// Cloneable handle to the per-tenant rate limiters.
#[derive(Clone)]
pub struct TenantRateLimiter {
    inner: Arc<Inner>,
}

struct Inner {
    buckets: Mutex<HashMap<String, Bucket>>,
    config: RateLimitConfig,
}

impl TenantRateLimiter {
    /// Build a limiter with `config`'s rate and burst.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                buckets: Mutex::new(HashMap::new()),
                config,
            }),
        }
    }

    /// Try to spend one token for `tenant`. Returns `Ok(())` if admitted, or
    /// `Err(retry_after)` with the duration until the next token if the bucket
    /// is empty. A new tenant starts with a full bucket.
    pub async fn check(&self, tenant: &str) -> Result<(), Duration> {
        let now = Instant::now();
        let mut buckets = self.inner.buckets.lock().await;
        let bucket = buckets.entry(tenant.to_string()).or_insert(Bucket {
            tokens: self.inner.config.burst,
            last_refill: now,
        });

        // Lazily refill for the time elapsed since the last check, capped at the
        // burst ceiling, then try to spend one token.
        let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.inner.config.refill_per_sec)
            .min(self.inner.config.burst);
        bucket.last_refill = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            let deficit = 1.0 - bucket.tokens;
            let secs = deficit / self.inner.config.refill_per_sec;
            Err(Duration::from_secs_f64(secs))
        }
    }
}

/// Parse a floating-point environment variable, treating absent/unparseable as
/// `None`.
fn env_f64(key: &str) -> Option<f64> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn burst_is_spent_then_throttled() {
        // A bucket of 3 with a slow refill: three pass, the fourth is throttled.
        let limiter = TenantRateLimiter::new(RateLimitConfig {
            refill_per_sec: 1.0,
            burst: 3.0,
        });
        for _ in 0..3 {
            limiter.check("t1").await.expect("within burst");
        }
        let throttled = limiter.check("t1").await;
        assert!(throttled.is_err(), "fourth exceeds the burst");
    }

    #[tokio::test]
    async fn refill_restores_tokens_over_time() {
        let limiter = TenantRateLimiter::new(RateLimitConfig {
            refill_per_sec: 100.0,
            burst: 1.0,
        });
        limiter.check("t1").await.expect("first token");
        assert!(limiter.check("t1").await.is_err(), "bucket emptied");
        // 100 tokens/sec means one token back in ~10ms; wait comfortably past it.
        tokio::time::sleep(Duration::from_millis(30)).await;
        limiter.check("t1").await.expect("refilled");
    }

    #[tokio::test]
    async fn tenants_have_independent_buckets() {
        let limiter = TenantRateLimiter::new(RateLimitConfig {
            refill_per_sec: 0.001,
            burst: 1.0,
        });
        limiter.check("a").await.expect("a's only token");
        assert!(limiter.check("a").await.is_err(), "a is drained");
        limiter.check("b").await.expect("b unaffected by a");
    }
}
