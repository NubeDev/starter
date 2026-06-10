//! Per-tenant query concurrency caps (WS-09 P1).
//!
//! The query guards bound a *single* query (read-only, timeout, row/byte caps),
//! but nothing today bounds how many queries one tenant runs at once. A single
//! dashboard with N panels on a fast refresh can occupy every connection in the
//! pool and starve other tenants. This caps the number of *concurrent* queries
//! per tenant with a semaphore: past the cap, a query is rejected with a clear,
//! retryable error rather than queuing unboundedly behind the pool.
//!
//! Caps sit *in front of* the query guards — they never loosen them.

mod limiter;

pub use limiter::{ConcurrencyGuard, QuotaConfig, TenantQuotas};
