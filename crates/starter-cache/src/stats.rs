//! Hit/miss counters. Kept in a tiny dedicated module so backends
//! can opt in without dragging metrics machinery into the trait.

use std::sync::atomic::{AtomicU64, Ordering};

/// Lightweight, lock-free hit/miss counters.
///
/// Backends are free to maintain a `CacheStats` and expose it; the
/// [`Cache`](crate::Cache) trait deliberately does not require it,
/// because not every backend (e.g. a future Valkey one) has cheap
/// local stats.
#[derive(Debug, Default)]
pub struct CacheStats {
    hits: AtomicU64,
    misses: AtomicU64,
}

impl CacheStats {
    /// Fresh zeroed counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a hit.
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a miss.
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Total hits observed.
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Total misses observed.
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// `hits / (hits + misses)`. Returns `0.0` before any access.
    pub fn hit_ratio(&self) -> f64 {
        let h = self.hits() as f64;
        let m = self.misses() as f64;
        let total = h + m;
        if total == 0.0 { 0.0 } else { h / total }
    }
}
