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

    /// Force eviction of expired entries. Useful in tests and on
    /// diagnostic endpoints; production code can rely on moka's
    /// background sweeping.
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
        // Flag flipped inside the loader so we can attribute the call
        // to hit vs miss without a second probe. moka's `try_get_with`
        // runs `init` at most once per key per process, and only on a
        // real miss.
        let miss = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let miss_writer = Arc::clone(&miss);
        let result = self
            .inner
            .try_get_with(key, async move {
                miss_writer.store(true, std::sync::atomic::Ordering::Relaxed);
                init().await
            })
            .await;

        match result {
            Ok(v) => {
                if miss.load(std::sync::atomic::Ordering::Relaxed) {
                    self.stats.record_miss();
                } else {
                    self.stats.record_hit();
                }
                Ok(v)
            }
            Err(arc_err) => {
                // Loader failed; count as a miss (we did the work, it
                // just didn't produce a usable value).
                self.stats.record_miss();
                match Arc::try_unwrap(arc_err) {
                    Ok(e) => Err(CacheError::Loader(e)),
                    Err(arc) => {
                        tracing::debug!(
                            "moka try_get_with: shared error Arc still held by {} waiters",
                            Arc::strong_count(&arc)
                        );
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
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "loader must run exactly once"
        );

        // Stats: exactly one miss (the single loader run); the rest
        // are hits served from the cached value.
        let s = cache.stats();
        assert_eq!(s.misses(), 1, "single-flight: only one miss");
        assert_eq!(s.hits(), 15, "remaining callers must be hits");
    }
}
