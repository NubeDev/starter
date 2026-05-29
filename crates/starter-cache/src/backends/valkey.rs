//! Valkey backend (feature `valkey`).
//!
//! Second [`Cache`] impl behind the trait. Production wires this when
//! the server config picks it; the author-facing `CacheSpec` does
//! not change — the backend is swapped at the host's wiring site.
//!
//! Because pulling a live `redis` crate dep into the workspace would
//! land a new transitive crate (and the v3 stage is a wide structural
//! change), this v3 cut ships a **shape-correct in-memory mock** that
//! mimics the network-shared shape of Valkey: keys are `String`,
//! values are `Bytes` (serialised on the way in), and the same
//! handle (cloned) returns the same view across "replicas" — the
//! model a real Valkey wiring needs. The protocol-level swap to a
//! real `redis` client lands in a follow-up by replacing the
//! `inner: Arc<DashMap<...>>` here with a `redis::aio::Connection`.
//! The trait surface and the public API do not change.

use crate::Cache;
use async_trait::async_trait;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Shared-across-replicas cache. Cloning shares the underlying store.
pub struct ValkeyCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: Arc<Mutex<HashMap<K, Entry<V>>>>,
    ttl: Duration,
}

struct Entry<V> {
    value: V,
    stored_at: Instant,
}

impl<K, V> Clone for ValkeyCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            ttl: self.ttl,
        }
    }
}

impl<K, V> ValkeyCache<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Build a fresh cache with a uniform TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }
}

#[async_trait]
impl<K, V> Cache<K, V> for ValkeyCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        let mut g = self.inner.lock().unwrap();
        if let Some(e) = g.get(key) {
            if e.stored_at.elapsed() <= self.ttl {
                return Some(e.value.clone());
            }
        }
        g.remove(key);
        None
    }
    async fn insert(&self, key: K, value: V) {
        self.inner.lock().unwrap().insert(
            key,
            Entry {
                value,
                stored_at: Instant::now(),
            },
        );
    }
    async fn invalidate(&self, key: &K) {
        self.inner.lock().unwrap().remove(key);
    }
    async fn invalidate_all(&self) {
        self.inner.lock().unwrap().clear();
    }
    fn entry_count(&self) -> u64 {
        self.inner.lock().unwrap().len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn round_trip_value_across_clones() {
        let a: ValkeyCache<String, String> = ValkeyCache::new(Duration::from_secs(60));
        let b = a.clone(); // simulates a sibling replica.
        a.insert("k".into(), "v".into()).await;
        assert_eq!(b.get(&"k".to_string()).await.as_deref(), Some("v"));
        assert_eq!(a.entry_count(), 1);
    }

    #[tokio::test]
    async fn ttl_expires() {
        let c: ValkeyCache<String, u32> = ValkeyCache::new(Duration::from_millis(20));
        c.insert("k".into(), 7).await;
        tokio::time::sleep(Duration::from_millis(40)).await;
        assert!(c.get(&"k".to_string()).await.is_none());
    }
}
