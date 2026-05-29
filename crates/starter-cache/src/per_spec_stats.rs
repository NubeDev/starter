//! Per-spec hit/miss counters.
//!
//! The aggregate [`CacheStats`](crate::CacheStats) is enough for "is
//! the layer doing anything"; the v0 canary needs per-kind numbers to
//! say "is `usage_bucketed` paying off". The dispatcher names each
//! cached call by a string id (e.g. `"com.nubeio.rubixos::usage_bucketed"`)
//! and the layer tallies hits/misses against it.
//!
//! Concurrency: a single `Mutex<HashMap<String, Arc<CacheStats>>>`.
//! Stats lookups happen once per dispatch — the cache layer caches
//! the `Arc<CacheStats>` for the duration of the call so the lock is
//! taken once per first-seen spec id, not per access.

use crate::CacheStats;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Per-spec stats registry. Cheap to clone — internally `Arc`-shared.
#[derive(Debug, Clone, Default)]
pub struct PerSpecStats {
    inner: Arc<Mutex<HashMap<String, Arc<CacheStats>>>>,
}

impl PerSpecStats {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up (or lazily create) the stats handle for `spec_id`.
    /// Cheap on repeated calls — one `HashMap::get` + `Arc::clone`.
    pub fn get_or_create(&self, spec_id: &str) -> Arc<CacheStats> {
        let mut g = self.inner.lock().expect("per-spec stats poisoned");
        if let Some(s) = g.get(spec_id) {
            return Arc::clone(s);
        }
        let s = Arc::new(CacheStats::new());
        g.insert(spec_id.to_string(), Arc::clone(&s));
        s
    }

    /// Snapshot every spec's current counters. Returned vector is
    /// sorted by spec id for stable admin output.
    pub fn snapshot(&self) -> Vec<PerSpecSnapshot> {
        let g = self.inner.lock().expect("per-spec stats poisoned");
        let mut out: Vec<PerSpecSnapshot> = g
            .iter()
            .map(|(id, s)| PerSpecSnapshot {
                spec_id: id.clone(),
                hits: s.hits(),
                misses: s.misses(),
                hit_ratio: s.hit_ratio(),
            })
            .collect();
        out.sort_by(|a, b| a.spec_id.cmp(&b.spec_id));
        out
    }

    /// Forget every spec. Useful in tests; in production the registry
    /// grows monotonically with the kind set, which is bounded by the
    /// number of registered sidecars.
    pub fn reset(&self) {
        self.inner.lock().expect("per-spec stats poisoned").clear();
    }
}

/// One row of the per-spec snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct PerSpecSnapshot {
    /// The opaque id the caller assigned this spec.
    pub spec_id: String,
    /// Hits since the registry was created (or reset).
    pub hits: u64,
    /// Misses since the registry was created (or reset).
    pub misses: u64,
    /// `hits / (hits + misses)`, or 0 before any access.
    pub hit_ratio: f64,
}
