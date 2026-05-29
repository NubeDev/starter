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
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Aggregated load-latency tally for one spec.
///
/// Six fixed-edge buckets (≤10ms, ≤100ms, ≤1s, ≤10s, >10s) plus
/// a total count and a sum of nanoseconds. Lock-free
/// (`AtomicU64` per bucket); cheap on the hot path.
///
/// Fixed buckets keep the wire shape stable and remove the need
/// for a histogram crate dep; the buckets are picked to bracket
/// the dashboard workload (sub-100ms = cheap, sub-1s = expected,
/// >1s = expensive and the cache is paying for itself).
#[derive(Debug, Default)]
pub struct LoadLatency {
    /// `<= 10ms`.
    pub le_10ms: AtomicU64,
    /// `<= 100ms` (and `> 10ms`).
    pub le_100ms: AtomicU64,
    /// `<= 1s`.
    pub le_1s: AtomicU64,
    /// `<= 10s`.
    pub le_10s: AtomicU64,
    /// `> 10s`.
    pub gt_10s: AtomicU64,
    /// Total samples.
    pub count: AtomicU64,
    /// Sum of every sample in nanoseconds — overflow is not a
    /// concern in practice (1e9 samples of 1s = 30 yrs).
    pub sum_nanos: AtomicU64,
}

impl LoadLatency {
    /// Record one sample.
    pub fn record(&self, d: Duration) {
        let nanos = d.as_nanos() as u64;
        self.sum_nanos.fetch_add(nanos, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        let ms = d.as_millis();
        if ms <= 10 {
            self.le_10ms.fetch_add(1, Ordering::Relaxed);
        } else if ms <= 100 {
            self.le_100ms.fetch_add(1, Ordering::Relaxed);
        } else if ms <= 1_000 {
            self.le_1s.fetch_add(1, Ordering::Relaxed);
        } else if ms <= 10_000 {
            self.le_10s.fetch_add(1, Ordering::Relaxed);
        } else {
            self.gt_10s.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Snapshot the histogram.
    pub fn snapshot(&self) -> LoadLatencySnapshot {
        LoadLatencySnapshot {
            le_10ms: self.le_10ms.load(Ordering::Relaxed),
            le_100ms: self.le_100ms.load(Ordering::Relaxed),
            le_1s: self.le_1s.load(Ordering::Relaxed),
            le_10s: self.le_10s.load(Ordering::Relaxed),
            gt_10s: self.gt_10s.load(Ordering::Relaxed),
            count: self.count.load(Ordering::Relaxed),
            sum_nanos: self.sum_nanos.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of [`LoadLatency`] at a point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadLatencySnapshot {
    /// `<= 10ms`.
    pub le_10ms: u64,
    /// `<= 100ms`.
    pub le_100ms: u64,
    /// `<= 1s`.
    pub le_1s: u64,
    /// `<= 10s`.
    pub le_10s: u64,
    /// `> 10s`.
    pub gt_10s: u64,
    /// Total samples.
    pub count: u64,
    /// Sum of every sample in nanoseconds.
    pub sum_nanos: u64,
}

impl LoadLatencySnapshot {
    /// Mean sample latency, or zero if no samples.
    pub fn mean(&self) -> Duration {
        if self.count == 0 {
            return Duration::ZERO;
        }
        Duration::from_nanos(self.sum_nanos / self.count)
    }
}

/// Per-spec counters + latency histogram.
#[derive(Debug, Default)]
pub struct SpecCounters {
    /// Hit/miss counters (same shape as the global [`CacheStats`]).
    pub stats: CacheStats,
    /// Histogram of loader-call latencies. Only updated on a miss
    /// (a hit's "load" cost is the moka get + token check, which is
    /// negligible and would dilute the signal the cache exists to
    /// quantify).
    pub load: LoadLatency,
}

impl SpecCounters {
    /// Record a hit (does not touch the load histogram).
    pub fn record_hit(&self) {
        self.stats.record_hit();
    }

    /// Record a miss with the loader's wall-clock duration.
    pub fn record_miss(&self, load_duration: Duration) {
        self.stats.record_miss();
        self.load.record(load_duration);
    }
}

/// Per-spec stats registry. Cheap to clone — internally `Arc`-shared.
#[derive(Debug, Clone, Default)]
pub struct PerSpecStats {
    inner: Arc<Mutex<HashMap<String, Arc<SpecCounters>>>>,
}

impl PerSpecStats {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Look up (or lazily create) the counters handle for `spec_id`.
    /// Cheap on repeated calls — one `HashMap::get` + `Arc::clone`.
    pub fn get_or_create(&self, spec_id: &str) -> Arc<SpecCounters> {
        let mut g = self.inner.lock().expect("per-spec stats poisoned");
        if let Some(s) = g.get(spec_id) {
            return Arc::clone(s);
        }
        let s = Arc::new(SpecCounters::default());
        g.insert(spec_id.to_string(), Arc::clone(&s));
        s
    }

    /// Snapshot every spec's current counters. Returned vector is
    /// sorted by spec id for stable admin output.
    pub fn snapshot(&self) -> Vec<PerSpecSnapshot> {
        let g = self.inner.lock().expect("per-spec stats poisoned");
        let mut out: Vec<PerSpecSnapshot> = g
            .iter()
            .map(|(id, c)| PerSpecSnapshot {
                spec_id: id.clone(),
                hits: c.stats.hits(),
                misses: c.stats.misses(),
                hit_ratio: c.stats.hit_ratio(),
                load_latency: c.load.snapshot(),
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
    /// Loader-call latency histogram. Only miss-path samples are
    /// recorded; hits are near-instant and excluding them keeps the
    /// histogram a measure of what the cache is shielding callers
    /// from.
    pub load_latency: LoadLatencySnapshot,
}
