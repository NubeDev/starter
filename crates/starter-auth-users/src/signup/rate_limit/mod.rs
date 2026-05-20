//! Rate-limiting seam for signup. Checked **before** password hashing
//! to prevent Argon2id CPU-DoS (R6).

mod memory;

use std::net::IpAddr;

use async_trait::async_trait;

pub use memory::MemoryRateLimiter;

/// Returned when a rate-limit bucket is exhausted.
#[derive(Debug, Clone)]
pub struct RateLimited {
    /// Seconds the client should wait before retrying.
    pub retry_after_secs: u32,
}

/// Trait seam for signup rate limiting. Implementations must be
/// thread-safe (`Send + Sync`).
#[async_trait]
pub trait SignupRateLimiter: Send + Sync {
    /// Check whether the request is allowed. Returns `Ok(())` if the
    /// request may proceed, or `Err(RateLimited)` with the
    /// `Retry-After` value otherwise.
    async fn check(&self, ip: IpAddr, email_normalised: &str) -> Result<(), RateLimited>;
}

/// No-op rate limiter for tests and consumers behind a WAF.
pub struct NoRateLimit;

#[async_trait]
impl SignupRateLimiter for NoRateLimit {
    async fn check(&self, _ip: IpAddr, _email_normalised: &str) -> Result<(), RateLimited> {
        Ok(())
    }
}
