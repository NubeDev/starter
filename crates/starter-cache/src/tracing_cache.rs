//! `TracingCache<C>` — a thin wrapper that records every cache event
//! for test assertions.
//!
//! Used in the v0 test suite to verify "the store was dropped" / "the
//! miss path ran" without taking a dependency on the underlying
//! moka cache's stats.

use crate::{Cache, CacheError};
use async_trait::async_trait;
use std::collections::VecDeque;
use std::future::Future;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// One observed cache operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEvent<K: Clone, V: Clone> {
    /// `get` returned `Some(_)`.
    Hit(K, V),
    /// `get` returned `None`.
    Miss(K),
    /// `insert` happened.
    Store(K, V),
    /// `invalidate` for a specific key.
    Drop(K),
}

/// Wrapper that records cache events.
pub struct TracingCache<C, K, V>
where
    C: Cache<K, V>,
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    inner: C,
    events: Arc<Mutex<VecDeque<CacheEvent<K, V>>>>,
}

impl<C, K, V> TracingCache<C, K, V>
where
    C: Cache<K, V>,
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Wrap an existing cache.
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            events: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Drain every recorded event in insertion order.
    pub fn drain_events(&self) -> Vec<CacheEvent<K, V>> {
        let mut g = self.events.lock().unwrap();
        g.drain(..).collect()
    }
}

#[async_trait]
impl<C, K, V> Cache<K, V> for TracingCache<C, K, V>
where
    C: Cache<K, V>,
    K: Hash + Eq + Send + Sync + Clone + 'static,
    V: Clone + Send + Sync + 'static,
{
    async fn get(&self, key: &K) -> Option<V> {
        let v = self.inner.get(key).await;
        let mut g = self.events.lock().unwrap();
        match &v {
            Some(value) => g.push_back(CacheEvent::Hit(key.clone(), value.clone())),
            None => g.push_back(CacheEvent::Miss(key.clone())),
        }
        v
    }

    async fn insert(&self, key: K, value: V) {
        let event = CacheEvent::Store(key.clone(), value.clone());
        self.inner.insert(key, value).await;
        self.events.lock().unwrap().push_back(event);
    }

    async fn invalidate(&self, key: &K) {
        self.inner.invalidate(key).await;
        self.events
            .lock()
            .unwrap()
            .push_back(CacheEvent::Drop(key.clone()));
    }

    async fn invalidate_all(&self) {
        self.inner.invalidate_all().await;
    }

    fn entry_count(&self) -> u64 {
        self.inner.entry_count()
    }

    async fn get_or_insert_with<F, Fut, E>(&self, key: K, init: F) -> Result<V, CacheError<E>>
    where
        F: FnOnce() -> Fut + Send,
        Fut: Future<Output = Result<V, E>> + Send,
        E: Send + Sync + 'static,
    {
        // Defer to the inner backend's single-flight, but still
        // record a Hit / Miss before any insert so test assertions
        // see the call shape.
        if let Some(v) = self.inner.get(&key).await {
            self.events
                .lock()
                .unwrap()
                .push_back(CacheEvent::Hit(key.clone(), v.clone()));
            return Ok(v);
        }
        self.events
            .lock()
            .unwrap()
            .push_back(CacheEvent::Miss(key.clone()));
        let v = init().await.map_err(CacheError::Loader)?;
        let event = CacheEvent::Store(key.clone(), v.clone());
        self.inner.insert(key, v.clone()).await;
        self.events.lock().unwrap().push_back(event);
        Ok(v)
    }
}
