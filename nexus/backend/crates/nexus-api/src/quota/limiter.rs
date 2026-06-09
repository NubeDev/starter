//! A per-tenant concurrency limiter backed by one semaphore per tenant.
//!
//! Each tenant gets a semaphore with `max_concurrent` permits. A query acquires
//! a permit for the duration of its run and releases it on drop; if no permit is
//! free the acquire fails *immediately* (`try_acquire`) rather than queuing, so
//! one tenant's burst cannot grow an unbounded backlog that delays everyone. The
//! returned guard holds the permit until the query finishes.
//!
//! Single-node like the rest of the v1 hardening: each node enforces its own cap.

use std::collections::HashMap;
use std::sync::Arc;

use starter_spi::Error;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

/// Stable error code a transport keys "too busy, retry" UI off, rather than
/// parsing the message. Surfaces as HTTP 503 (the `Unavailable` class).
pub const QUOTA_EXCEEDED_CODE: &str = "quota.concurrency_exceeded";

/// Concurrency cap configuration.
#[derive(Debug, Clone, Copy)]
pub struct QuotaConfig {
    /// Maximum concurrent queries one tenant may run on this node.
    pub max_concurrent: usize,
}

impl Default for QuotaConfig {
    fn default() -> Self {
        Self { max_concurrent: 16 }
    }
}

impl QuotaConfig {
    /// Read `NEXUS_TENANT_MAX_CONCURRENT_QUERIES` from the environment, keeping
    /// the default for an absent/unparseable/zero value (a zero cap would lock
    /// every tenant out — clearly not intended).
    pub fn from_env() -> Self {
        let default = Self::default();
        let max_concurrent = std::env::var("NEXUS_TENANT_MAX_CONCURRENT_QUERIES")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|c| *c > 0)
            .unwrap_or(default.max_concurrent);
        Self { max_concurrent }
    }
}

/// Cloneable handle to the per-tenant concurrency limiters.
#[derive(Clone)]
pub struct TenantQuotas {
    inner: Arc<Inner>,
}

struct Inner {
    semaphores: Mutex<HashMap<String, Arc<Semaphore>>>,
    config: QuotaConfig,
}

/// Held for the lifetime of a query; releasing it (on drop) returns the permit.
/// The wrapped permit is the only state — the guard exists so the call site
/// keeps the permit alive for exactly the query's duration via RAII.
pub struct ConcurrencyGuard {
    _permit: OwnedSemaphorePermit,
}

impl TenantQuotas {
    /// Build limiters with `config`'s per-tenant cap.
    pub fn new(config: QuotaConfig) -> Self {
        Self {
            inner: Arc::new(Inner {
                semaphores: Mutex::new(HashMap::new()),
                config,
            }),
        }
    }

    /// Try to admit one query for `tenant`. Returns a guard the caller holds for
    /// the query's duration, or `Error::Unavailable` if the tenant is already at
    /// its concurrency cap. Other tenants are unaffected — each has its own
    /// semaphore — so one tenant hitting its cap never throttles another.
    pub async fn admit(&self, tenant: &str) -> Result<ConcurrencyGuard, Error> {
        let semaphore = self.semaphore_for(tenant).await;
        match semaphore.try_acquire_owned() {
            Ok(permit) => Ok(ConcurrencyGuard { _permit: permit }),
            Err(_) => Err(Error::Unavailable {
                code: QUOTA_EXCEEDED_CODE.to_string(),
                subject: Some(tenant.to_string()),
                message: format!(
                    "tenant query concurrency limit ({}) reached; retry shortly",
                    self.inner.config.max_concurrent
                ),
            }),
        }
    }

    /// Return (creating on first touch) the semaphore for `tenant`. A new
    /// tenant's semaphore starts with the full permit count.
    async fn semaphore_for(&self, tenant: &str) -> Arc<Semaphore> {
        let mut map = self.inner.semaphores.lock().await;
        map.entry(tenant.to_string())
            .or_insert_with(|| Arc::new(Semaphore::new(self.inner.config.max_concurrent)))
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn admits_up_to_the_cap_then_rejects() {
        let quotas = TenantQuotas::new(QuotaConfig { max_concurrent: 2 });
        let g1 = quotas.admit("t1").await.expect("first admitted");
        let _g2 = quotas.admit("t1").await.expect("second admitted");
        // The third exceeds the cap while the first two are held.
        let third = quotas.admit("t1").await;
        assert!(matches!(third, Err(Error::Unavailable { .. })));
        // Releasing one frees a permit for the next.
        drop(g1);
        quotas.admit("t1").await.expect("admitted after release");
    }

    #[tokio::test]
    async fn one_tenant_at_cap_does_not_throttle_another() {
        let quotas = TenantQuotas::new(QuotaConfig { max_concurrent: 1 });
        let _held = quotas.admit("busy").await.expect("busy admitted");
        assert!(quotas.admit("busy").await.is_err(), "busy is capped");
        // A different tenant has its own semaphore and is unaffected.
        quotas
            .admit("other")
            .await
            .expect("other tenant unaffected by busy's cap");
    }
}
