//! In-memory token-bucket rate limiter. Default implementation of
//! [`SignupRateLimiter`].
//!
//! Two buckets per request: one keyed by IP, one by normalised email.
//! Whichever has less remaining budget determines `Retry-After`.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::Instant;

use async_trait::async_trait;

use super::{RateLimited, SignupRateLimiter};

/// Token bucket configuration.
const MAX_TOKENS: u32 = 5;
const REFILL_INTERVAL_SECS: u64 = 600; // 10 minutes

/// A single token bucket.
#[derive(Debug, Clone)]
struct Bucket {
    tokens: u32,
    last_refill: Instant,
}

impl Bucket {
    fn new() -> Self {
        Self {
            tokens: MAX_TOKENS,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time, then try to consume one.
    /// Returns the remaining retry-after seconds on failure.
    fn try_consume(&mut self) -> Result<(), u32> {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs();
        // Refill one token per (interval / max_tokens) seconds.
        let refill_rate_secs = REFILL_INTERVAL_SECS / u64::from(MAX_TOKENS);
        let refilled = (elapsed / refill_rate_secs) as u32;
        if refilled > 0 {
            self.tokens = (self.tokens + refilled).min(MAX_TOKENS);
            self.last_refill = now;
        }

        if self.tokens > 0 {
            self.tokens -= 1;
            Ok(())
        } else {
            // Seconds until next token.
            let next_refill = refill_rate_secs.saturating_sub(elapsed % refill_rate_secs);
            Err(next_refill as u32)
        }
    }
}

/// In-memory token-bucket rate limiter. 5 requests per 10 minutes per
/// IP and per normalised email.
pub struct MemoryRateLimiter {
    ip_buckets: Mutex<HashMap<IpAddr, Bucket>>,
    email_buckets: Mutex<HashMap<String, Bucket>>,
}

impl MemoryRateLimiter {
    /// Create a new limiter with empty buckets.
    pub fn new() -> Self {
        Self {
            ip_buckets: Mutex::new(HashMap::new()),
            email_buckets: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MemoryRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SignupRateLimiter for MemoryRateLimiter {
    async fn check(&self, ip: IpAddr, email_normalised: &str) -> Result<(), RateLimited> {
        let ip_result = {
            let mut buckets = self.ip_buckets.lock().unwrap_or_else(|e| e.into_inner());
            let bucket = buckets.entry(ip).or_insert_with(Bucket::new);
            bucket.try_consume()
        };

        let email_result = {
            let mut buckets = self.email_buckets.lock().unwrap_or_else(|e| e.into_inner());
            let bucket = buckets
                .entry(email_normalised.to_owned())
                .or_insert_with(Bucket::new);
            bucket.try_consume()
        };

        match (ip_result, email_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(ip_secs), Err(email_secs)) => Err(RateLimited {
                retry_after_secs: ip_secs.max(email_secs),
            }),
            (Err(secs), Ok(())) | (Ok(()), Err(secs)) => Err(RateLimited {
                retry_after_secs: secs,
            }),
        }
    }
}
