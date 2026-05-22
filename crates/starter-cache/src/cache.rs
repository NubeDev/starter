//! The [`Cache`] trait — every backend implements this and nothing
//! else. Call sites depend on the trait, never on a concrete backend.

use crate::error::CacheError;
use async_trait::async_trait;
use std::{future::Future, hash::Hash};

/// Generic async cache surface.
///
/// Bounds match what every reasonable backend (in-process or
/// network-shared) needs: hashable keys, cloneable values, both
/// `Send + Sync + 'static` so handlers can share the cache across
/// tasks.
#[async_trait]
pub trait Cache<K, V>: Send + Sync
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Fetch a value by key. Returns `None` on miss or expiry.
    async fn get(&self, key: &K) -> Option<V>;

    /// Insert (or overwrite) a value.
    async fn insert(&self, key: K, value: V);

    /// Drop a single key. No-op if absent.
    async fn invalidate(&self, key: &K);

    /// Drop every entry.
    async fn invalidate_all(&self);

    /// Approximate entry count. Cheap; not exact under concurrent
    /// eviction.
    fn entry_count(&self) -> u64;

    /// Stampede-safe load. If `key` is present, returns the cached
    /// value. Otherwise calls `init` **exactly once** across all
    /// concurrent callers for the same key; other callers wait on
    /// the in-flight load.
    ///
    /// This is the primitive for "cache the rendered page" — without
    /// it, a cache miss under load fires the loader N times.
    ///
    /// Backends that cannot offer single-flight semantics natively
    /// must still satisfy the "exactly once per key per process"
    /// contract (e.g. via an internal `tokio::sync::Mutex` map). The
    /// default impl is **not** single-flight; backends should
    /// override.
    async fn get_or_insert_with<F, Fut, E>(&self, key: K, init: F) -> Result<V, CacheError<E>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<V, E>> + Send,
        E: Send + Sync + 'static,
    {
        if let Some(v) = self.get(&key).await {
            return Ok(v);
        }
        let v = init().await.map_err(CacheError::Loader)?;
        self.insert(key, v.clone()).await;
        Ok(v)
    }
}
