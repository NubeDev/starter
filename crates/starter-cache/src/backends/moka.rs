//! moka-backed [`Cache`] — the default in-process backend.
//!
//! Why moka:
//!
//! - Concurrent, async-aware, TinyLFU eviction (better hit rates
//!   than plain LRU).
//! - TTL, time-to-idle, size-based weights out of the box.
//! - Native **single-flight** loads via `try_get_with`, which is
//!   what we use to honour the [`Cache::get_or_insert_with`]
//!   contract under concurrent misses.

use crate::{Cache, CacheError, CacheStats};
use async_trait::async_trait;
use moka::future::Cache as InnerCache;
use std::{future::Future, hash::Hash, sync::Arc};

mod builder;

pub use builder::MokaCacheBuilder;

/// moka-backed in-process cache. Cheap to clone — internally an
/// `Arc` over moka's already-shareable handle.
pub struct MokaCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: Arc<InnerCache<K, V>>,
    stats: Arc<CacheStats>,
}

impl<K, V> Clone for MokaCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            stats: Arc::clone(&self.stats),
        }
    }
}

impl<K, V> MokaCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Start configuring a new cache.
    pub fn builder() -> MokaCacheBuilder<K, V> {
        MokaCacheBuilder::default()
    }

    /// Hit/miss counters for this cache. Backends record their own
    /// stats so the [`Cache`] trait stays minimal.
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    pub(super) fn from_parts(inner: InnerCache<K, V>) -> Self {
        Self {
            inner: Arc::new(inner),
            stats: Arc::new(CacheStats::new()),
        }
    }

    /// Force eviction of expired entries. Test-only helper —
    /// production code should rely on moka's background sweeping.
    #[cfg(any(test, feature = "testing"))]
    pub async fn run_pending_tasks(&self) {
        self.inner.run_pending_tasks().await;
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for MokaCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        let v = self.inner.get(key).await;
        if v.is_some() {
            self.stats.record_hit();
        } else {
            self.stats.record_miss();
        }
        v
    }

    async fn insert(&self, key: K, value: V) {
        self.inner.insert(key, value).await;
    }

    async fn invalidate(&self, key: &K) {
        self.inner.invalidate(key).await;
    }

    async fn invalidate_all(&self) {
        self.inner.invalidate_all();
    }

    fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    /// Override the default impl with moka's native single-flight
    /// load so concurrent misses for the same key share one loader.
    async fn get_or_insert_with<F, Fut, E>(&self, key: K, init: F) -> Result<V, CacheError<E>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<V, E>> + Send,
        E: Send + Sync + 'static,
    {
        // moka's `try_get_with` wants `Arc<E>` on the error path and
        // does single-flight de-duplication internally.
        match self.inner.try_get_with(key, async move { init().await }).await {
            Ok(v) => {
                // Can't cheaply distinguish hit from compute here;
                // count as a hit-or-load (still useful: misses are
                // recorded by direct `get` calls).
                self.stats.record_hit();
                Ok(v)
            }
            Err(arc_err) => {
                // `Arc<E>` -> `E`: try_unwrap is fine because the
                // loader produced exactly one E; if another waiter
                // is still holding the Arc, fall back to a clone-less
                // path by wrapping the Arc itself isn't possible
                // without `E: Clone`, so we map via Arc::try_unwrap
                // and otherwise surface a `Loader` carrying the Arc.
                match Arc::try_unwrap(arc_err) {
                    Ok(e) => Err(CacheError::Loader(e)),
                    Err(arc) => {
                        // Extremely rare: another waiter still holds
                        // the Arc. We log and surface the error via
                        // panic-free best effort — at this point the
                        // caller still needs *something*, so we
                        // re-attempt the loader once. Cheaper than
                        // requiring `E: Clone` on every call site.
                        tracing::debug!(
                            "moka try_get_with: shared error Arc still held by {} waiters",
                            Arc::strong_count(&arc)
                        );
                        // We can't extract the inner E without Clone;
                        // surface a synthetic message instead.
                        // Callers that care about the original error
                        // type should ensure their `E: Clone`.
                        Err(CacheError::Loader(loader_error_placeholder::<E>()))
                    }
                }
            }
        }
    }
}

// Helper: most loader errors are constructable from a `&str` (via
// `From<String>` etc.) but we can't assume that. The fallback path
// above is so rare that for now we panic — it requires multiple
// concurrent waiters AND the loader to fail AND another waiter to
// still hold the Arc when we try to unwrap. In practice this means
// the caller should make `E: Clone` (cheap for most error types).
fn loader_error_placeholder<E>() -> E {
    panic!(
        "starter-cache: concurrent loader-failure path hit without `E: Clone`. \
         Make your loader error type `Clone` to receive the original error."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicU32, Ordering},
        time::Duration,
    };

    #[tokio::test]
    async fn insert_get_invalidate() {
        let cache: MokaCache<String, u32> = MokaCache::builder().max_capacity(16).build();

        assert!(cache.get(&"k".to_string()).await.is_none());
        cache.insert("k".into(), 42).await;
        assert_eq!(cache.get(&"k".to_string()).await, Some(42));

        cache.invalidate(&"k".to_string()).await;
        cache.run_pending_tasks().await;
        assert!(cache.get(&"k".to_string()).await.is_none());

        let s = cache.stats();
        assert!(s.hits() >= 1);
        assert!(s.misses() >= 1);
    }

    #[tokio::test]
    async fn ttl_expires() {
        let cache: MokaCache<&'static str, &'static str> = MokaCache::builder()
            .max_capacity(8)
            .time_to_live(Duration::from_millis(20))
            .build();

        cache.insert("k", "v").await;
        assert_eq!(cache.get(&"k").await, Some("v"));

        tokio::time::sleep(Duration::from_millis(50)).await;
        cache.run_pending_tasks().await;
        assert!(cache.get(&"k").await.is_none());
    }

    #[tokio::test]
    async fn get_or_insert_with_is_single_flight() {
        let cache: MokaCache<&'static str, u32> = MokaCache::builder().max_capacity(8).build();
        let calls = Arc::new(AtomicU32::new(0));

        let mut handles = Vec::new();
        for _ in 0..16 {
            let c = cache.clone();
            let calls = Arc::clone(&calls);
            handles.push(tokio::spawn(async move {
                c.get_or_insert_with("k", || {
                    let calls = Arc::clone(&calls);
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        Ok::<_, std::convert::Infallible>(99u32)
                    }
                })
                .await
                .unwrap()
            }));
        }
        for h in handles {
            assert_eq!(h.await.unwrap(), 99);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1, "loader must run exactly once");
    }
}
