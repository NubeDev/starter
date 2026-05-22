//! No-op backend. Every `get` misses, every `insert` is dropped,
//! `entry_count` is always zero.
//!
//! Useful for:
//!
//! - Tests that want to assert "the loader actually ran".
//! - Wiring "cache disabled" via config without forking call sites.
//!
//! `get_or_insert_with` still runs the loader and returns its value
//! — that's the contract — it just never caches it.

use crate::Cache;
use async_trait::async_trait;
use std::{hash::Hash, marker::PhantomData};

/// Always-miss cache.
pub struct NoopCache<K, V>(PhantomData<fn() -> (K, V)>);

impl<K, V> Default for NoopCache<K, V> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<K, V> NoopCache<K, V> {
    /// Create a new no-op cache.
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for NoopCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, _key: &K) -> Option<V> {
        None
    }
    async fn insert(&self, _key: K, _value: V) {}
    async fn invalidate(&self, _key: &K) {}
    async fn invalidate_all(&self) {}
    fn entry_count(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn never_caches() {
        let c: NoopCache<&'static str, u32> = NoopCache::new();
        c.insert("k", 1).await;
        assert!(c.get(&"k").await.is_none());
        assert_eq!(c.entry_count(), 0);
    }

    #[tokio::test]
    async fn get_or_insert_with_still_runs_loader() {
        let c: NoopCache<&'static str, u32> = NoopCache::new();
        let v = c
            .get_or_insert_with("k", || async { Ok::<_, std::convert::Infallible>(7) })
            .await
            .unwrap();
        assert_eq!(v, 7);
        // Still nothing cached.
        assert!(c.get(&"k").await.is_none());
    }
}
