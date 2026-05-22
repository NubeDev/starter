//! Builder for [`MokaCache`]. In its own file so the main backend
//! module stays focused on the `Cache` impl.

use super::MokaCache;
use moka::future::Cache as InnerCache;
use std::{hash::Hash, marker::PhantomData, time::Duration};

/// Builder for [`MokaCache`]. Mirrors the subset of moka's builder
/// surface we want to expose. Add knobs here as call sites need them
/// — don't pre-emptively re-export the whole moka API.
pub struct MokaCacheBuilder<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    max_capacity: Option<u64>,
    time_to_live: Option<Duration>,
    time_to_idle: Option<Duration>,
    _marker: PhantomData<fn() -> (K, V)>,
}

impl<K, V> Default for MokaCacheBuilder<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            max_capacity: None,
            time_to_live: None,
            time_to_idle: None,
            _marker: PhantomData,
        }
    }
}

impl<K, V> MokaCacheBuilder<K, V>
where
    K: Hash + Eq + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Hard upper bound on entry count. moka uses TinyLFU to evict
    /// past this.
    pub fn max_capacity(mut self, n: u64) -> Self {
        self.max_capacity = Some(n);
        self
    }

    /// Evict entries this long after **insertion**.
    pub fn time_to_live(mut self, ttl: Duration) -> Self {
        self.time_to_live = Some(ttl);
        self
    }

    /// Evict entries this long after the **last access**.
    pub fn time_to_idle(mut self, tti: Duration) -> Self {
        self.time_to_idle = Some(tti);
        self
    }

    /// Materialise the cache.
    pub fn build(self) -> MokaCache<K, V> {
        let mut b = InnerCache::<K, V>::builder();
        if let Some(n) = self.max_capacity {
            b = b.max_capacity(n);
        }
        if let Some(ttl) = self.time_to_live {
            b = b.time_to_live(ttl);
        }
        if let Some(tti) = self.time_to_idle {
            b = b.time_to_idle(tti);
        }
        MokaCache::from_parts(b.build())
    }
}
