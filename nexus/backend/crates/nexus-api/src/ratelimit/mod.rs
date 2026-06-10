//! Per-tenant request rate limiting (WS-09 P1).
//!
//! A token-bucket limiter, applied as middleware, caps the request rate per
//! tenant so one tenant cannot saturate the server and starve others. Unlike the
//! concurrency quota (which bounds *in-flight* queries), this bounds the *rate*
//! of requests over time, smoothing bursts. Each tenant refills tokens at a
//! steady rate up to a burst ceiling; a request with no token available is
//! rejected with HTTP 429 and a `Retry-After` hint.
//!
//! Single-node like the rest of the v1 hardening: each node enforces its own
//! per-tenant rate.

mod bucket;
mod layer;

pub use bucket::{RateLimitConfig, TenantRateLimiter};
pub use layer::rate_limit_layer;
