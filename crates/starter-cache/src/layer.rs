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

/// What a loader produced for one miss.
///
/// `Empty` is an explicit marker — the layer treats it as cacheable
/// under the spec's `empty_ttl`, distinct from `Value(Bytes)` which
/// uses the regular TTL. Callers wrap `Empty` rather than relying on
/// `Vec::is_empty()` because a `"[]"` JSON body is not the same as
/// a logically empty result.
#[derive(Debug, Clone)]
pub enum LoadOutcome {
    /// Non-empty answer.
    Value(Bytes),
    /// Logically empty answer (no rows, no data). Stored under
    /// `empty_ttl` and only if the spec's `cache_empty` is true.
    Empty,
}

impl LoadOutcome {
    /// Convenience: build a `Value` from bytes.
    pub fn value(b: Bytes) -> Self {
        Self::Value(b)
    }
    /// Materialise as bytes — `Empty` becomes an empty `Vec<u8>`.
    /// Callers that need to distinguish should pattern-match
    /// directly.
    pub fn into_bytes(self) -> Bytes {
        match self {
            Self::Value(b) => b,
            Self::Empty => Arc::new(Vec::new()),
        }
    }
    /// `true` if this outcome is `Empty`.
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

/// Stored entry: the value plus the snapshot we took at load-start,
/// the load instant for SWR computations, and a refresh marker.
#[derive(Clone)]
struct StoredEntry {
    value: Bytes,
    snapshot: TokenSnapshot,
    loaded_at: std::time::Instant,
    is_empty: bool,
    /// Set once the layer has served the entry as a stale SWR hit;
    /// the next caller treats the entry as a miss to drive a refresh.
    /// `Arc<AtomicBool>` keeps `StoredEntry: Clone` while the flag
    /// stays shared across moka's internal clones.
    needs_refresh: Arc<std::sync::atomic::AtomicBool>,
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

    /// Drop every cached entry across every tenant. Last-resort
    /// operator escape hatch for "the cache is in a bad state, drop
    /// it all and let it warm again". Tags survive (tokens are not
    /// reset); per-spec counters survive (so the operator can watch
    /// hit rate recover from a known baseline).
    ///
    /// Returns the total approximate count dropped.
    pub async fn invalidate_all(&self) -> u64 {
        let caches: Vec<TenantCache> = {
            let g = self.inner.tenants.lock().unwrap();
            g.values().cloned().collect()
        };
        let mut total: u64 = 0;
        for cache in caches {
            total = total.saturating_add(cache.entry_count());
            cache.invalidate_all().await;
            cache.run_pending_tasks().await;
        }
        total
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
        // Adapt the bytes-only loader into a LoadOutcome-returning
        // one (always `Value`). Empty marker semantics need the
        // outcome-aware entry point.
        let outcome = self
            .get_or_load_labelled_outcome(spec, spec_id, caller, base_key, || async move {
                load().await.map(LoadOutcome::Value)
            })
            .await?;
        Ok(outcome.into_bytes())
    }

    /// Outcome-aware entry point. The loader returns
    /// [`LoadOutcome`] so the layer can distinguish empty results
    /// from non-empty ones and apply the spec's `empty_ttl` /
    /// `cache_empty` accordingly. Also the canonical SWR entry
    /// point: when a cached entry's age falls inside the spec's
    /// `stale_while_revalidate` window (or after expiry but within
    /// `max_stale`), the cached value is served as a hit and the
    /// entry is marked for refresh; the next caller for the same
    /// key sees the marker, treats the entry as a miss, and reloads.
    ///
    /// This is the v1 SWR implementation. True background-spawned
    /// refresh (single-flight via a 'static refresher) is deferred
    /// to v3 alongside the `WarehouseWriter` chokepoint, since both
    /// want the same `'static` refresher abstraction.
    pub async fn get_or_load_labelled_outcome<F, Fut, E>(
        &self,
        spec: &CacheSpec,
        spec_id: Option<&str>,
        caller: &CallerScope,
        base_key: &str,
        load: F,
    ) -> Result<LoadOutcome, E>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<LoadOutcome, E>> + Send,
        E: Send + Sync + 'static,
    {
        // For TTL-based moka eviction, use the larger of ttl and
        // ttl+max_stale so the moka cache itself doesn't expire an
        // entry the layer would still serve via SWR.
        let outer_ttl = spec.ttl.saturating_add(spec.max_stale);
        let bucket = Self::tenant_bucket(spec.scope, caller);
        let cache = self.tenant_cache(&bucket, outer_ttl);
        let key = Self::full_key(spec.scope, caller, base_key);
        let per_spec = spec_id.map(|id| self.inner.per_spec.get_or_create(id));

        // Read path.
        if let Some(entry) = cache.get(&key).await {
            if self.inner.invalidator.tokens_match(&entry.snapshot) {
                let now = self.inner.clock.now();
                let age = now.saturating_duration_since(entry.loaded_at);
                let effective_ttl = if entry.is_empty {
                    spec.empty_ttl
                } else {
                    spec.ttl
                };
                // Empty entries get neither SWR nor max_stale —
                // they expire hard at `empty_ttl`, otherwise a noisy
                // empty-cell would linger far past its useful life.
                let (fresh_until, stale_limit) = if entry.is_empty {
                    (effective_ttl, effective_ttl)
                } else {
                    (
                        effective_ttl.saturating_sub(spec.stale_while_revalidate),
                        effective_ttl.saturating_add(spec.max_stale),
                    )
                };

                if age < fresh_until {
                    // Fresh hit.
                    self.inner.stats.record_hit();
                    if let Some(s) = &per_spec {
                        s.record_hit();
                    }
                    return Ok(self.outcome_from(&entry));
                } else if age < stale_limit {
                    // Inside SWR window or past TTL but inside
                    // max_stale. Serve stale; the first caller
                    // marks the entry, the next caller drives the
                    // refresh.
                    let already_marked = entry
                        .needs_refresh
                        .swap(true, std::sync::atomic::Ordering::SeqCst);
                    if !already_marked {
                        self.inner.stats.record_hit();
                        if let Some(s) = &per_spec {
                            s.record_hit();
                        }
                        tracing::debug!(
                            "starter-cache: SWR stale-serve for {key:?} (age={:?})",
                            age
                        );
                        return Ok(self.outcome_from(&entry));
                    }
                    // Marker already set — fall through to miss
                    // path. The current caller becomes the
                    // refresher.
                    cache.invalidate(&key).await;
                } else {
                    // Past max_stale — hard miss.
                    cache.invalidate(&key).await;
                }
            } else {
                tracing::debug!("starter-cache: serving miss for {key:?} — stored snapshot stale");
                cache.invalidate(&key).await;
            }
        }

        // Miss path.
        let tags = spec.derived_tags();
        let snap_before = self.inner.invalidator.snapshot_tokens(&tags);
        let load_started = self.inner.clock.now();
        let outcome = load().await?;
        let load_duration = self
            .inner
            .clock
            .now()
            .saturating_duration_since(load_started);

        // Decide what to store. `Empty` only goes in when
        // `cache_empty` is on.
        let (store_bytes, is_empty) = match &outcome {
            LoadOutcome::Value(b) => (Some(b.clone()), false),
            LoadOutcome::Empty => {
                if spec.cache_empty {
                    (Some(Arc::new(Vec::new())), true)
                } else {
                    (None, true)
                }
            }
        };

        if let Some(value) = store_bytes {
            // Race check.
            if self.inner.invalidator.tokens_match(&snap_before) {
                cache
                    .insert(
                        key,
                        StoredEntry {
                            value,
                            snapshot: snap_before,
                            loaded_at: self.inner.clock.now(),
                            is_empty,
                            needs_refresh: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        },
                    )
                    .await;
            } else {
                tracing::debug!(
                    "starter-cache: dropping store — \
                     invalidation token moved during load"
                );
            }
        }

        self.inner.stats.record_miss();
        if let Some(s) = &per_spec {
            s.record_miss(load_duration);
        }
        Ok(outcome)
    }

    /// v2 — two-layer cache adapter (§Layer 6c).
    ///
    /// When the spec has `inner_scope: Some(_)` and `scope: User`,
    /// this entry point caches the **canonical-units** loader output
    /// at `inner_scope` (typically `tenant`) and the rendered output
    /// at `scope` (`user`). The caller passes `render` to convert
    /// canonical bytes to user-rendered bytes — the conversion is
    /// expected to use the host's `starter-i18n` / `starter-prefs`
    /// units stack against the user's preferences (the layer doesn't
    /// link those crates directly; the closure carries the
    /// dependency).
    ///
    /// When `inner_scope` is `None` (or when scope is not `User`),
    /// this is equivalent to a single-layer
    /// [`Self::get_or_load_labelled`] at `spec.scope`.
    pub async fn get_or_load_two_layer<F, FFut, R, RFut, E>(
        &self,
        spec: &CacheSpec,
        spec_id: Option<&str>,
        caller: &CallerScope,
        base_key: &str,
        canonical_load: F,
        render: R,
    ) -> Result<Bytes, E>
    where
        F: FnOnce() -> FFut + Send,
        FFut: Future<Output = Result<Bytes, E>> + Send,
        R: FnOnce(Bytes) -> RFut + Send,
        RFut: Future<Output = Result<Bytes, E>> + Send,
        E: Send + Sync + 'static,
    {
        // Single-layer fast path.
        let Some(inner_scope) = spec.inner_scope else {
            return self
                .get_or_load_labelled(spec, spec_id, caller, base_key, canonical_load)
                .await;
        };
        if !matches!(spec.scope, CacheScope::User) {
            // inner_scope only meaningful when the outer scope is User.
            return self
                .get_or_load_labelled(spec, spec_id, caller, base_key, canonical_load)
                .await;
        }

        // Outer (user-scope) lookup first.
        let outer_key = Self::full_key(spec.scope, caller, base_key);
        let outer_bucket = Self::tenant_bucket(spec.scope, caller);
        let outer_ttl = spec.ttl.saturating_add(spec.max_stale);
        let outer_cache = self.tenant_cache(&outer_bucket, outer_ttl);
        if let Some(entry) = outer_cache.get(&outer_key).await {
            if self.inner.invalidator.tokens_match(&entry.snapshot) {
                let now = self.inner.clock.now();
                let age = now.saturating_duration_since(entry.loaded_at);
                if age < spec.ttl {
                    self.inner.stats.record_hit();
                    return Ok(entry.value.clone());
                }
                outer_cache.invalidate(&outer_key).await;
            } else {
                outer_cache.invalidate(&outer_key).await;
            }
        }

        // Outer miss — drop through to inner (tenant-scope) lookup,
        // then render, then store outer.
        let inner_spec = {
            let mut s = spec.clone();
            s.scope = inner_scope;
            s.inner_scope = None;
            s
        };
        let inner_caller = match inner_scope {
            CacheScope::Tenant => CallerScope {
                tenant: caller.tenant.clone(),
                user: None,
            },
            CacheScope::Global => CallerScope::system(),
            CacheScope::User => caller.clone(),
        };
        let canonical = self
            .get_or_load_labelled(
                &inner_spec,
                spec_id,
                &inner_caller,
                base_key,
                canonical_load,
            )
            .await?;

        let rendered = render(canonical).await?;

        // Store outer.
        let tags = spec.derived_tags();
        let snap = self.inner.invalidator.snapshot_tokens(&tags);
        outer_cache
            .insert(
                outer_key,
                StoredEntry {
                    value: rendered.clone(),
                    snapshot: snap,
                    loaded_at: self.inner.clock.now(),
                    is_empty: false,
                    needs_refresh: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                },
            )
            .await;
        self.inner.stats.record_miss();
        Ok(rendered)
    }

    /// v2 — windowed delta-fetch adapter (§Layer 4b).
    ///
    /// Decomposes the `[from, to]` window into per-bucket sub-fetches
    /// against the supplied [`starter_windowed::WindowedFetcher`].
    /// Each closed bucket is cached with `body_ttl` semantics; the
    /// tail bucket uses `tail_ttl`. The request's `to` is snapped to
    /// the bucket boundary before it enters the cache key so per-bucket
    /// key cardinality is bounded.
    ///
    /// Bucket-level invalidation tags
    /// (`bucket:<table>:<floor(t,bucket)>`) are wired through the
    /// existing invalidator: each per-bucket store subscribes to the
    /// exact `bucket:<table>:<bucket-rfc3339>` tag the writer would
    /// fire — one write touches at most one cached bucket per
    /// subscribed table.
    pub async fn get_or_load_windowed<T>(
        &self,
        spec: &CacheSpec,
        spec_id: Option<&str>,
        caller: &CallerScope,
        base_key: &str,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
        fetcher: &dyn starter_windowed::WindowedFetcher<T>,
    ) -> Result<T, starter_windowed::FetchError>
    where
        T: starter_windowed::Stitchable
            + serde::Serialize
            + serde::de::DeserializeOwned
            + Send
            + Sync
            + 'static,
    {
        let ws = spec.windowed_spec().ok_or_else(|| {
            starter_windowed::FetchError::Other("spec has no time_series block".into())
        })?;
        let buckets = starter_windowed::decompose(&ws, from, to);
        if buckets.is_empty() {
            return Ok(T::stitch(Vec::new()));
        }

        let per_spec = spec_id.map(|id| self.inner.per_spec.get_or_create(id));
        let bucket_scope = spec.inner_scope.unwrap_or(spec.scope);
        let bucket_caller = match bucket_scope {
            CacheScope::Tenant => CallerScope {
                tenant: caller.tenant.clone(),
                user: None,
            },
            CacheScope::Global => CallerScope::system(),
            CacheScope::User => caller.clone(),
        };
        let cache_bucket = Self::tenant_bucket(bucket_scope, &bucket_caller);
        // Use the larger of body/tail TTL so moka itself doesn't
        // expire underneath us.
        let moka_ttl = ws.body_ttl.max(ws.tail_ttl).saturating_add(spec.max_stale);
        let cache = self.tenant_cache(&cache_bucket, moka_ttl);

        // Per-table bucket-fire subscription tags. One per
        // (table, bucket-key) pair.
        let mut parts: Vec<T> = Vec::with_capacity(buckets.len());
        for b in buckets {
            let key_suffix = b.key();
            let bucket_table_tags: Vec<String> = spec
                .invalidate_on
                .tables
                .iter()
                .map(|t| format!("bucket:{t}:{key_suffix}"))
                .collect();
            let mut all_tags = spec.derived_tags();
            all_tags.extend(bucket_table_tags.iter().cloned());

            let full_key = Self::full_key(
                bucket_scope,
                &bucket_caller,
                &format!("{base_key}::bucket::{key_suffix}"),
            );

            // Read path.
            let mut hit = false;
            let mut hit_value: Option<T> = None;
            if let Some(entry) = cache.get(&full_key).await {
                if self.inner.invalidator.tokens_match(&entry.snapshot) {
                    let now = self.inner.clock.now();
                    let age = now.saturating_duration_since(entry.loaded_at);
                    let ttl = if b.is_tail { ws.tail_ttl } else { ws.body_ttl };
                    if age < ttl {
                        if let Ok(v) = serde_json::from_slice::<T>(&entry.value) {
                            hit = true;
                            hit_value = Some(v);
                        }
                    }
                }
                if !hit {
                    cache.invalidate(&full_key).await;
                }
            }
            if let Some(v) = hit_value {
                self.inner.stats.record_hit();
                if let Some(s) = &per_spec {
                    s.record_hit();
                }
                parts.push(v);
                continue;
            }

            // Miss path.
            let snap_before = self.inner.invalidator.snapshot_tokens(&all_tags);
            let load_started = self.inner.clock.now();
            let value = fetcher.fetch_bucket(b.clone()).await?;
            let load_duration = self
                .inner
                .clock
                .now()
                .saturating_duration_since(load_started);
            let bytes = serde_json::to_vec(&value).map_err(|e| {
                starter_windowed::FetchError::Other(format!("serialize bucket: {e}"))
            })?;
            if self.inner.invalidator.tokens_match(&snap_before) {
                cache
                    .insert(
                        full_key,
                        StoredEntry {
                            value: Arc::new(bytes),
                            snapshot: snap_before,
                            loaded_at: self.inner.clock.now(),
                            is_empty: false,
                            needs_refresh: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                        },
                    )
                    .await;
            }
            self.inner.stats.record_miss();
            if let Some(s) = &per_spec {
                s.record_miss(load_duration);
            }
            parts.push(value);
        }
        Ok(T::stitch(parts))
    }

    fn outcome_from(&self, entry: &StoredEntry) -> LoadOutcome {
        if entry.is_empty {
            LoadOutcome::Empty
        } else {
            LoadOutcome::Value(entry.value.clone())
        }
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
        let spec = CacheSpec::ttl(Duration::from_secs(60)).invalidate_on_table("readings");
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
                .get_or_load::<_, _, std::convert::Infallible>(&spec, caller, "k", || async {
                    Ok(b("v"))
                })
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
        assert_eq!(layer.tenant_entry_count("tB"), 1, "B must be untouched");

        // A subsequent get_or_load for tA must pay a miss.
        let calls = Arc::new(AtomicU32::new(0));
        let calls2 = calls.clone();
        let _ = layer
            .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller_a, "k", || async move {
                calls2.fetch_add(1, Ordering::SeqCst);
                Ok(b("v2"))
            })
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
    async fn invalidate_all_drops_every_tenants_entries() {
        let layer = CacheLayer::new(LayerConfig::default());
        let spec = CacheSpec::ttl(Duration::from_secs(60)).scope(CacheScope::Tenant);

        for tenant in ["tA", "tB", "tC"] {
            let caller = CallerScope::new(tenant, "u");
            let _ = layer
                .get_or_load::<_, _, std::convert::Infallible>(&spec, &caller, "k", || async {
                    Ok(b("v"))
                })
                .await
                .unwrap();
        }
        layer.run_pending_tasks().await;
        let total = layer.invalidate_all().await;
        assert_eq!(total, 3);
        for tenant in ["tA", "tB", "tC"] {
            assert_eq!(layer.tenant_entry_count(tenant), 0);
        }
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
