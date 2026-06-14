//! Cache sizing, read from the environment with safe defaults.
//!
//! The TTL should track the dashboard refresh interval: an entry only needs to
//! survive long enough to serve a tick's worth of identical repeats. Too long
//! and a manual refresh shows stale rows past the next tick; the default of a
//! few seconds matches the fastest refresh the picker offers.

use std::time::Duration;

use super::QueryCache;

/// Tunable cache bounds.
#[derive(Debug, Clone, Copy)]
pub struct CacheConfig {
    /// How long a cached result stays live. Aligned to the refresh interval.
    pub ttl: Duration,
    /// Maximum number of live entries before the soonest-to-expire is evicted.
    pub capacity: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(10),
            capacity: 10_000,
        }
    }
}

impl CacheConfig {
    /// Read overrides from `NEXUS_QUERY_CACHE_TTL_SECS` and
    /// `NEXUS_QUERY_CACHE_CAPACITY`, falling back to the defaults. A
    /// zero/unparseable value keeps the default rather than disabling the cache
    /// by accident — disabling is a per-request concern (`refresh=off`), not a
    /// global one.
    pub fn from_env() -> Self {
        let default = Self::default();
        let ttl = env_u64("NEXUS_QUERY_CACHE_TTL_SECS")
            .filter(|s| *s > 0)
            .map(Duration::from_secs)
            .unwrap_or(default.ttl);
        let capacity = env_u64("NEXUS_QUERY_CACHE_CAPACITY")
            .filter(|c| *c > 0)
            .map(|c| c as usize)
            .unwrap_or(default.capacity);
        Self { ttl, capacity }
    }

    /// Build a cache from this config.
    pub fn build(&self) -> QueryCache {
        QueryCache::new(self.ttl, self.capacity)
    }
}

/// Parse an unsigned integer environment variable, treating absent/unparseable
/// as `None`.
fn env_u64(key: &str) -> Option<u64> {
    std::env::var(key).ok().and_then(|v| v.parse().ok())
}
