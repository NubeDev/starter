//! `CacheLayer` — the v0 opt-in cache wrapper.
//!
//! A call site (today: the extension kind dispatcher) hands the layer
//! a [`CacheSpec`], a [`CallerScope`], a key, and a loader closure.
//! The layer:
//!
//! 1. Derives a per-tenant [`MokaCache`] (creating one on first use),
//!    sized by the layer's per-tenant cap. This is how the proposal's
//!    "per-tenant weight caps via moka weight-based eviction" lands
//!    in v0 — partitioning by tenant is the simplest correct
//!    implementation (each tenant gets its own moka cache, so a
//!    noisy tenant evicts only its own entries).
//! 2. Derives a full cache key from `(scope, tenant, user, base_key)`.
//! 3. Reads the invalidator's token snapshot for the spec's tags
//!    **before** firing the loader, stores the snapshot alongside
//!    the loaded value, and rechecks it on both store and read:
//!    a mid-load invalidation drops the store; a tag fired after
//!    storing makes the entry served as a miss.
//!
//! Step 3 is the invalidation-race fix the v0 proposal pins as
//! non-negotiable.

use crate::backends::moka::MokaCache;
use crate::clock::{Clock, SystemClock};
use crate::invalidator::{InMemoryInvalidator, Invalidator, TokenSnapshot};
use crate::per_spec_stats::{PerSpecSnapshot, PerSpecStats};
use crate::spec::{CacheScope, CacheSpec};
use crate::stats::CacheStats;
use crate::Cache;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};

/// Identity bits the layer mixes into the key. `tenant` and `user`
/// are opaque strings — the layer makes no assumption about their
/// shape, only that two distinct identities never collide as
/// strings.
#[derive(Debug, Clone)]
pub struct CallerScope {
    /// Tenant id, or `None` for system / host-internal frames.
    pub tenant: Option<String>,
    /// User id, or `None` for unauthenticated requests.
    pub user: Option<String>,
}

impl CallerScope {
    /// Build with a tenant and user id.
    pub fn new(tenant: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            tenant: Some(tenant.into()),
            user: Some(user.into()),
        }
    }
    /// System / host-internal frame.
    pub fn system() -> Self {
        Self {
            tenant: None,
            user: None,
        }
    }
}

/// v0 configuration for the layer.
#[derive(Debug, Clone)]
pub struct LayerConfig {
    /// Per-tenant entry cap. Each tenant gets a moka cache sized to
    /// this; one noisy tenant cannot evict another's entries because
    /// they live in physically separate caches.
    pub per_tenant_max_entries: u64,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            per_tenant_max_entries: 10_000,
        }
    }
}

impl LayerConfig {
    /// Build a config by reading host env vars, falling back to the
    /// default for any var that is unset or unparseable.
    ///
    /// `prefix` is the env-var prefix the host uses (e.g.
    /// `"RUBIX_CACHE"`). Read keys:
    ///
    /// - `<prefix>_PER_TENANT_MAX_ENTRIES` — integer, > 0.
    ///
    /// Unparseable values fall through to the default; the layer
    /// does not panic on bad operator input. Callers that want to
    /// observe parse failures can pre-parse via `std::env::var(...)`
    /// before calling this and emit their own warning.
    pub fn from_env(prefix: &str) -> Self {
        let mut cfg = Self::default();
        let key = format!("{prefix}_PER_TENANT_MAX_ENTRIES");
        if let Ok(raw) = std::env::var(&key) {
            if let Ok(n) = raw.parse::<u64>() {
                if n > 0 {
                    cfg.per_tenant_max_entries = n;
                }
            }
        }
        cfg
    }
}

/// Cached payload type. We pin to `Arc<Vec<u8>>` so any
/// JSON-serialisable response can be cached behind one shape.
pub type Bytes = Arc<Vec<u8>>;

/// Stored entry: the value plus the snapshot we took at load-start.
#[derive(Clone)]
struct StoredEntry {
    value: Bytes,
    snapshot: TokenSnapshot,
}

type TenantCache = MokaCache<String, StoredEntry>;

/// The cache layer. Cheap to clone — internally `Arc`-shared.
#[derive(Clone)]
pub struct CacheLayer {
    inner: Arc<Inner>,
}

struct Inner {
    config: LayerConfig,
    /// Threaded through to time loader calls (per-spec load
    /// histogram) and for future expiry tracking. The default is
    /// [`SystemClock`]; tests wire in [`crate::MockClock`].
    clock: Arc<dyn Clock>,
    invalidator: Arc<dyn Invalidator>,
    tenants: Mutex<HashMap<String, TenantCache>>,
    stats: Arc<CacheStats>,
    per_spec: PerSpecStats,
}

impl CacheLayer {
    /// Build a layer with the default [`SystemClock`] and an
    /// in-memory invalidator.
    pub fn new(config: LayerConfig) -> Self {
        Self::with_parts(
            config,
            Arc::new(SystemClock),
            Arc::new(InMemoryInvalidator::new()),
        )
    }

    /// Build a layer with custom clock and invalidator (test path).
    pub fn with_parts(
        config: LayerConfig,
        clock: Arc<dyn Clock>,
        invalidator: Arc<dyn Invalidator>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                config,
                clock,
                invalidator,
                tenants: Mutex::new(HashMap::new()),
                stats: Arc::new(CacheStats::new()),
                per_spec: PerSpecStats::new(),
            }),
        }
    }

    /// Hit/miss counters across every tenant cache.
    pub fn stats(&self) -> &CacheStats {
        &self.inner.stats
    }

    /// Borrow the invalidator (write-path hooks call
    /// `invalidate_tags` through it).
    pub fn invalidator(&self) -> Arc<dyn Invalidator> {
        Arc::clone(&self.inner.invalidator)
    }

    fn full_key(scope: CacheScope, caller: &CallerScope, base_key: &str) -> String {
        match scope {
            CacheScope::Global => format!("g::{base_key}"),
            CacheScope::Tenant => format!(
                "t::{}::{}",
                caller.tenant.as_deref().unwrap_or("__system"),
                base_key,
            ),
            CacheScope::User => format!(
                "u::{}::{}::{}",
                caller.tenant.as_deref().unwrap_or("__system"),
                caller.user.as_deref().unwrap_or("__anon"),
                base_key,
            ),
        }
    }

    fn tenant_bucket(scope: CacheScope, caller: &CallerScope) -> String {
        match scope {
            CacheScope::Global => "__global".to_string(),
            CacheScope::Tenant | CacheScope::User => caller
                .tenant
                .clone()
                .unwrap_or_else(|| "__system".to_string()),
        }
    }

    fn tenant_cache(&self, bucket: &str, ttl: std::time::Duration) -> TenantCache {
        let mut g = self.inner.tenants.lock().unwrap();
        if let Some(c) = g.get(bucket) {
            return c.clone();
        }
        let cache = MokaCache::<String, StoredEntry>::builder()
            .max_capacity(self.inner.config.per_tenant_max_entries)
            .time_to_live(ttl)
            .build();
        g.insert(bucket.to_string(), cache.clone());
        cache
    }

    /// Entry count for a tenant's cache (test/observability helper).
    pub fn tenant_entry_count(&self, tenant: &str) -> u64 {
        let g = self.inner.tenants.lock().unwrap();
        g.get(tenant).map(|c| c.entry_count()).unwrap_or(0)
    }

    /// Snapshot of every tenant id with a currently-allocated cache
    /// (a cache is allocated lazily on first access). Sorted by id
    /// for stable admin output.
    pub fn tenant_ids(&self) -> Vec<String> {
        let g = self.inner.tenants.lock().unwrap();
        let mut ids: Vec<String> = g.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Drop every cached entry belonging to `tenant`. Useful when a
    /// tenant is disabled or deleted and the operator wants to
    /// reclaim its cache memory immediately rather than waiting for
    /// process restart. No-op when the tenant has no allocated
    /// cache.
    ///
    /// The synthetic tenant ids — `"__global"` (for `scope: global`
    /// entries) and `"__system"` (for system-frame entries) — are
    /// accepted by the same path; passing them is intentional but
    /// unusual.
    ///
    /// Returns the number of entries dropped (approximate, since
    /// moka's `entry_count` is eventually consistent).
    pub async fn evict_tenant(&self, tenant: &str) -> u64 {
        let cache = {
            let g = self.inner.tenants.lock().unwrap();
            g.get(tenant).cloned()
        };
        let Some(cache) = cache else {
            return 0;
        };
        let before = cache.entry_count();
        cache.invalidate_all().await;
        cache.run_pending_tasks().await;
        before
    }

    /// Force every tenant cache's pending eviction tasks to drain.
    /// moka's entry counts are otherwise an eventually consistent
    /// estimate; useful in tests and on diagnostic endpoints.
    pub async fn run_pending_tasks(&self) {
        let caches: Vec<TenantCache> = {
            let g = self.inner.tenants.lock().unwrap();
            g.values().cloned().collect()
        };
        for c in caches {
            c.run_pending_tasks().await;
        }
    }

    /// Snapshot of per-spec stats. Sorted by `spec_id`.
    pub fn per_spec_snapshot(&self) -> Vec<PerSpecSnapshot> {
        self.inner.per_spec.snapshot()
    }

    /// Borrow the per-spec stats registry — useful for tests and for
    /// admin endpoints that want to wire in their own snapshot
    /// cadence.
    pub fn per_spec_stats(&self) -> PerSpecStats {
        self.inner.per_spec.clone()
    }

    /// The v0 entry point. Returns the cached or freshly loaded
    /// bytes. Equivalent to
    /// [`Self::get_or_load_labelled`] with `spec_id = None`;
    /// callers that already know which spec they are evaluating
    /// should use the labelled variant so per-spec stats are
    /// recorded.
    ///
    /// `base_key` should encode whatever identifies the underlying
    /// query — for kind dispatch, the dispatcher passes
    /// `format!("{ext}::{contribute_id}::{input_hash}")`.
    pub async fn get_or_load<F, Fut, E>(
        &self,
        spec: &CacheSpec,
        caller: &CallerScope,
        base_key: &str,
        load: F,
    ) -> Result<Bytes, E>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<Bytes, E>> + Send,
        E: Send + Sync + 'static,
    {
        self.get_or_load_labelled(spec, None, caller, base_key, load)
            .await
    }

    /// Variant of [`Self::get_or_load`] that also tallies per-spec
    /// hit/miss counters under `spec_id`. Pass the same `spec_id` on
    /// every call for one logical spec — typically
    /// `format!("{extension}::{contribute_id}")` at the dispatcher
    /// site.
    pub async fn get_or_load_labelled<F, Fut, E>(
        &self,
        spec: &CacheSpec,
        spec_id: Option<&str>,
        caller: &CallerScope,
        base_key: &str,
        load: F,
    ) -> Result<Bytes, E>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<Bytes, E>> + Send,
        E: Send + Sync + 'static,
    {
        let bucket = Self::tenant_bucket(spec.scope, caller);
        let cache = self.tenant_cache(&bucket, spec.ttl);
        let key = Self::full_key(spec.scope, caller, base_key);

        // Per-spec stats handle, materialised once per call so the
        // mutex behind PerSpecStats is touched at most twice per
        // dispatch (once here, never again on the hot path).
        let per_spec = spec_id.map(|id| self.inner.per_spec.get_or_create(id));

        // Read path: hit only if a stored entry exists AND its
        // snapshot still matches. A token-moved entry is treated
        // as a miss — this closes the read-side of the
        // invalidation-race story.
        if let Some(entry) = cache.get(&key).await {
            if self.inner.invalidator.tokens_match(&entry.snapshot) {
                self.inner.stats.record_hit();
                if let Some(s) = &per_spec {
                    s.record_hit();
                }
                return Ok(entry.value);
            } else {
                tracing::debug!(
                    "starter-cache: serving miss for {key:?} — stored snapshot stale"
                );
                cache.invalidate(&key).await;
            }
        }

        // Snapshot tokens **before** firing the loader.
        let tags = spec.derived_tags();
        let snap_before = self.inner.invalidator.snapshot_tokens(&tags);
        let load_started = self.inner.clock.now();
        let value = load().await?;
        let load_duration = self
            .inner
            .clock
            .now()
            .saturating_duration_since(load_started);

        // Race check: drop the store if any depended-on tag moved
        // during the load. Otherwise insert with the snapshot we
        // observed at load-start.
        if self.inner.invalidator.tokens_match(&snap_before) {
            cache
                .insert(
                    key,
                    StoredEntry {
                        value: value.clone(),
                        snapshot: snap_before,
                    },
                )
                .await;
        } else {
            tracing::debug!(
                "starter-cache: dropping store — \
                 invalidation token moved during load"
            );
        }

        self.inner.stats.record_miss();
        if let Some(s) = &per_spec {
            s.record_miss(load_duration);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::MockClock;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    fn b(v: &str) -> Bytes {
        Arc::new(v.as_bytes().to_vec())
    }

    #[tokio::test]
    async fn hit_miss_basic_flow() {
        let layer = CacheLayer::new(LayerConfig::default());
        let spec = CacheSpec::ttl(Duration::from_secs(60))
            .scope(CacheScope::Tenant)
            .invalidate_on_table("readings");
        let caller = CallerScope::new("tA", "uX");
        let calls = Arc::new(AtomicU32::new(0));

        for _ in 0..3 {
            let calls = calls.clone();
            let _ = layer
                .get_or_load(&spec, &caller, "k1", || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, std::convert::Infallible>(b("v"))
                })
                .await
                .unwrap();
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(layer.stats().hits() >= 2);
        assert!(layer.stats().misses() >= 1);
    }

    #[tokio::test]
    async fn invalidate_drops_entry() {
        let layer = CacheLayer::new(LayerConfig::default());
        let spec = CacheSpec::ttl(Duration::from_secs(60))
            .invalidate_on_table("readings");
        let caller = CallerScope::new("tA", "uX");
        let calls = Arc::new(AtomicU32::new(0));

        for i in 0..2 {
            let calls = calls.clone();
            let _ = layer
                .get_or_load(&spec, &caller, "k1", || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, std::convert::Infallible>(b(&format!("v{i}")))
                })
                .await
                .unwrap();
            if i == 0 {
                layer
                    .invalidator()
                    .invalidate_tags(&["table:readings".to_string()])
                    .await;
            }
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn layer_config_from_env_reads_per_tenant_cap() {
        // Use a unique prefix per test so env-var state across tests
        // doesn't bleed. The `_TEST_<random>` suffix is enough — the
        // helper takes the prefix verbatim.
        let prefix = "STARTER_CACHE_TEST_OK";
        // SAFETY: `set_var` is unsafe-without-warning on 2024 toolchains;
        // tests run single-threaded for env mutation by convention here
        // (no other test touches this prefix).
        std::env::set_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"), "42");
        let cfg = LayerConfig::from_env(prefix);
        assert_eq!(cfg.per_tenant_max_entries, 42);
        std::env::remove_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"));
    }

    #[test]
    fn layer_config_from_env_falls_back_on_unparseable() {
        let prefix = "STARTER_CACHE_TEST_BAD";
        std::env::set_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"), "not-a-number");
        let cfg = LayerConfig::from_env(prefix);
        assert_eq!(
            cfg.per_tenant_max_entries,
            LayerConfig::default().per_tenant_max_entries
        );
        std::env::remove_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"));
    }

    #[test]
    fn layer_config_from_env_rejects_zero() {
        let prefix = "STARTER_CACHE_TEST_ZERO";
        std::env::set_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"), "0");
        let cfg = LayerConfig::from_env(prefix);
        // Zero is meaningless (no entries ever stored); the helper
        // falls through to the default rather than wedge the cache.
        assert_eq!(
            cfg.per_tenant_max_entries,
            LayerConfig::default().per_tenant_max_entries
        );
        std::env::remove_var(format!("{prefix}_PER_TENANT_MAX_ENTRIES"));
    }

    #[tokio::test]
    async fn evict_tenant_drops_only_that_tenants_entries() {
        let layer = CacheLayer::new(LayerConfig::default());
        let spec = CacheSpec::ttl(Duration::from_secs(60)).scope(CacheScope::Tenant);
        let caller_a = CallerScope::new("tA", "uA");
        let caller_b = CallerScope::new("tB", "uB");

        // Populate two tenants.
        for caller in [&caller_a, &caller_b] {
            let _ = layer
                .get_or_load::<_, _, std::convert::Infallible>(
                    &spec,
                    caller,
                    "k",
                    || async { Ok(b("v")) },
                )
                .await
                .unwrap();
        }
        layer.run_pending_tasks().await;
        assert!(layer.tenant_ids().contains(&"tA".to_string()));
        assert!(layer.tenant_ids().contains(&"tB".to_string()));
        assert_eq!(layer.tenant_entry_count("tA"), 1);
        assert_eq!(layer.tenant_entry_count("tB"), 1);

        // Evict only tenant A.
        let dropped = layer.evict_tenant("tA").await;
        assert_eq!(dropped, 1);
        assert_eq!(layer.tenant_entry_count("tA"), 0);
        assert_eq!(
            layer.tenant_entry_count("tB"),
            1,
            "B must be untouched"
        );

        // A subsequent get_or_load for tA must pay a miss.
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(
                &spec,
                &caller_a,
                "k",
                || async move {
                    calls2.fetch_add(1, Ordering::SeqCst);
                    Ok(b("v2"))
                },
            )
            .await
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn evict_unknown_tenant_returns_zero() {
        let layer = CacheLayer::new(LayerConfig::default());
        assert_eq!(layer.evict_tenant("never-seen").await, 0);
    }

    #[tokio::test]
    async fn mock_clock_threads_through() {
        let clock = MockClock::new();
        let _layer = CacheLayer::with_parts(
            LayerConfig::default(),
            Arc::new(clock.clone()),
            Arc::new(InMemoryInvalidator::new()),
        );
        clock.advance(Duration::from_secs(1));
    }
}
