//! In-process TTL cache for query results, with single-flight coalescing.
//!
//! A 20-panel dashboard on a 10s auto-refresh issues ~120 queries/second per
//! viewer; without a cache every one hits the source database. This cache holds
//! a query's rows for a short TTL aligned to the refresh interval so repeated
//! identical panel queries within a tick are served from memory.
//!
//! Two properties make it production-safe rather than a naive map:
//!
//! - **Single-flight.** When N requests miss the same key simultaneously (the
//!   thundering herd a dashboard refresh creates), exactly one runs the backing
//!   load and the rest await its result. This is the property that actually
//!   protects the database, not the cache hit ratio.
//! - **Bounded.** Entries expire by TTL and the map is capped; an oversized map
//!   evicts the oldest entries so a busy tenant cannot exhaust memory.
//!
//! Single-node by design (like the pool cache and live fan-out): each node holds
//! its own entries, which is correct — a Redis backend behind the same interface
//! is the multi-node story (WS-09 P3), not a change to callers.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use nexus_spi::dto::query::QueryResponse;
use starter_spi::Error;
use tokio::sync::{watch, Mutex};

use super::key::CacheKey;

/// A cached query result with its expiry instant. Cloned out on a hit (rows are
/// owned JSON, so a clone is a deep copy — acceptable for the bounded sizes the
/// query guards already enforce).
#[derive(Clone)]
struct Entry {
    response: QueryResponse,
    expires_at: Instant,
}

/// Hit/miss/coalesced counters, readable for metrics and tests. Monotonic and
/// lock-free so reading them never contends with cache operations.
#[derive(Default)]
pub struct CacheStats {
    /// Requests served from a live (unexpired) entry.
    pub hits: AtomicU64,
    /// Requests that found no live entry and ran the backing load.
    pub misses: AtomicU64,
    /// Requests that joined an in-flight load for the same key instead of
    /// starting their own — the thundering-herd suppression.
    pub coalesced: AtomicU64,
}

impl CacheStats {
    /// Snapshot the three counters. A test reads this to prove a second
    /// identical query hit rather than re-ran.
    pub fn snapshot(&self) -> (u64, u64, u64) {
        (
            self.hits.load(Ordering::Relaxed),
            self.misses.load(Ordering::Relaxed),
            self.coalesced.load(Ordering::Relaxed),
        )
    }
}

/// A coalescing slot: the first misser (the leader) runs the load and publishes
/// the result on this channel; every concurrent misser of the same key holds a
/// receiver and awaits the published value. A `watch` channel is used (not a
/// `OnceCell`) precisely because a follower must *await* the leader's result,
/// not poll for it — `None` is the not-yet-loaded state.
type Flight = watch::Receiver<Option<Result<QueryResponse, FlightError>>>;

/// The role a caller takes for a key's in-flight load: the leader runs the load
/// and publishes via the sender; a follower awaits the published value.
enum Flightboard {
    Leader(watch::Sender<Option<Result<QueryResponse, FlightError>>>),
    Follower(Flight),
}

/// Await the leader's published result on a follower's receiver. The channel
/// starts at `None`; the leader sends `Some(result)` exactly once. If the leader
/// is dropped before publishing (a panic), the channel closes and the follower
/// reports a transient internal error rather than hanging forever.
async fn wait_for_flight(rx: &mut Flight) -> Result<QueryResponse, FlightError> {
    // The first observed value may already be the published result (if the leader
    // finished between insert and our await) or the initial `None`.
    if let Some(result) = rx.borrow().clone() {
        return result;
    }
    while rx.changed().await.is_ok() {
        if let Some(result) = rx.borrow().clone() {
            return result;
        }
    }
    Err(FlightError::Internal("coalesced load dropped".into()))
}

/// A load error shareable across the coalesced waiters. `Error` is not `Clone`,
/// so the in-flight result carries the rendered message and a 4xx/5xx class; the
/// followers reconstruct a domain error of the right class. The leader returns
/// its original `Error` unchanged.
#[derive(Clone)]
enum FlightError {
    /// The caller's query was rejected (read-only violation, syntax, timeout).
    Invalid(String),
    /// A connection/transaction failure on our side.
    Internal(String),
}

impl FlightError {
    fn from(e: &Error) -> Self {
        match e {
            Error::Invalid { message } => FlightError::Invalid(message.clone()),
            other => FlightError::Internal(other.to_string()),
        }
    }

    fn into_error(self) -> Error {
        match self {
            FlightError::Invalid(message) => Error::Invalid { message },
            FlightError::Internal(message) => Error::Internal {
                source: Box::new(std::io::Error::other(message)),
            },
        }
    }
}

/// Cloneable handle to the shared cache. Cheap to clone (an `Arc`), so it rides
/// on `AppState` like the other handles.
#[derive(Clone)]
pub struct QueryCache {
    inner: Arc<Inner>,
}

struct Inner {
    entries: Mutex<HashMap<CacheKey, Entry>>,
    flights: Mutex<HashMap<CacheKey, Flight>>,
    ttl: Duration,
    capacity: usize,
    stats: CacheStats,
}

impl QueryCache {
    /// Build a cache with a per-entry `ttl` and a `capacity` ceiling on live
    /// entries. The TTL should track the refresh interval so an entry survives
    /// exactly long enough to serve a refresh tick's worth of repeats.
    pub fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            inner: Arc::new(Inner {
                entries: Mutex::new(HashMap::new()),
                flights: Mutex::new(HashMap::new()),
                ttl,
                capacity,
                stats: CacheStats::default(),
            }),
        }
    }

    /// The hit/miss/coalesced counters.
    pub fn stats(&self) -> &CacheStats {
        &self.inner.stats
    }

    /// Return the cached response for `key`, or run `load` to produce it,
    /// caching the result. Concurrent misses for the same key coalesce onto one
    /// `load`. A load error is returned to every coalesced waiter but is **not**
    /// cached — only successful results are stored, so a transient failure does
    /// not poison the key for the TTL.
    pub async fn get_or_load<F, Fut>(&self, key: CacheKey, load: F) -> Result<QueryResponse, Error>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<QueryResponse, Error>>,
    {
        if let Some(hit) = self.live_entry(&key).await {
            self.inner.stats.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(hit);
        }

        let tx = match self.join_flight(&key).await {
            Flightboard::Leader(tx) => tx,
            Flightboard::Follower(mut rx) => {
                // A concurrent miss already started the load; await its result.
                self.inner.stats.coalesced.fetch_add(1, Ordering::Relaxed);
                return wait_for_flight(&mut rx)
                    .await
                    .map_err(FlightError::into_error);
            }
        };

        // We are the leader: run the load, publish it to followers, store a
        // successful result, then retire the flight slot.
        self.inner.stats.misses.fetch_add(1, Ordering::Relaxed);
        let result = load().await;
        let shared = result
            .as_ref()
            .map(QueryResponse::clone)
            .map_err(FlightError::from);
        // Publish to any followers. A send error means every follower has already
        // dropped — harmless; the leader still returns its own result.
        let _ = tx.send(Some(shared));
        if let Ok(response) = &result {
            self.store(key.clone(), response.clone()).await;
        }
        self.inner.flights.lock().await.remove(&key);
        result
    }

    /// Return a live (unexpired) cloned entry, lazily dropping it if it has
    /// expired so the map does not accumulate stale entries on read.
    async fn live_entry(&self, key: &CacheKey) -> Option<QueryResponse> {
        let mut map = self.inner.entries.lock().await;
        match map.get(key) {
            Some(entry) if entry.expires_at > Instant::now() => Some(entry.response.clone()),
            Some(_) => {
                map.remove(key);
                None
            }
            None => None,
        }
    }

    /// Join (or create) the in-flight slot for `key`. The first caller becomes
    /// the leader and gets the channel sender; later callers become followers and
    /// get a receiver cloned from the leader's channel.
    async fn join_flight(&self, key: &CacheKey) -> Flightboard {
        let mut flights = self.inner.flights.lock().await;
        if let Some(existing) = flights.get(key) {
            return Flightboard::Follower(existing.clone());
        }
        let (tx, rx) = watch::channel(None);
        flights.insert(key.clone(), rx);
        Flightboard::Leader(tx)
    }

    /// Insert a successful response under `key`, evicting the soonest-to-expire
    /// entry first if the map is at capacity. Bounded-size eviction by expiry is
    /// a good-enough approximation of LRU for a short-TTL query cache and avoids
    /// a second index.
    async fn store(&self, key: CacheKey, response: QueryResponse) {
        let mut map = self.inner.entries.lock().await;
        if map.len() >= self.inner.capacity && !map.contains_key(&key) {
            if let Some(oldest) = map
                .iter()
                .min_by_key(|(_, e)| e.expires_at)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest);
            }
        }
        map.insert(
            key,
            Entry {
                response,
                expires_at: Instant::now() + self.inner.ttl,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_spi::dto::query::QueryStats;
    use std::sync::atomic::AtomicUsize;

    fn key(s: &str) -> CacheKey {
        // The cache treats the key as opaque, so a test can mint one from the
        // public constructor of a stand-in request — but it is simpler to round
        // a string through the real builder. We expose a small shim instead.
        CacheKey::for_test(s)
    }

    fn response(row_count: u64) -> QueryResponse {
        QueryResponse {
            columns: Vec::new(),
            rows: Vec::new(),
            stats: QueryStats {
                row_count,
                byte_count: 0,
                elapsed_ms: 0,
                truncated: false,
            },
        }
    }

    #[tokio::test]
    async fn second_identical_query_hits_the_cache() {
        let cache = QueryCache::new(Duration::from_secs(60), 100);
        let loads = Arc::new(AtomicUsize::new(0));

        for _ in 0..2 {
            let loads = loads.clone();
            cache
                .get_or_load(key("k"), || async move {
                    loads.fetch_add(1, Ordering::SeqCst);
                    Ok(response(1))
                })
                .await
                .unwrap();
        }

        assert_eq!(loads.load(Ordering::SeqCst), 1, "load ran once");
        let (hits, misses, _) = cache.stats().snapshot();
        assert_eq!((hits, misses), (1, 1));
    }

    #[tokio::test]
    async fn expired_entry_reloads() {
        let cache = QueryCache::new(Duration::from_millis(20), 100);
        cache
            .get_or_load(key("k"), || async { Ok(response(1)) })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(40)).await;
        cache
            .get_or_load(key("k"), || async { Ok(response(1)) })
            .await
            .unwrap();
        let (hits, misses, _) = cache.stats().snapshot();
        assert_eq!((hits, misses), (0, 2), "both ran after TTL expiry");
    }

    #[tokio::test]
    async fn concurrent_misses_coalesce_onto_one_load() {
        let cache = QueryCache::new(Duration::from_secs(60), 100);
        let loads = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(tokio::sync::Barrier::new(8));

        let mut handles = Vec::new();
        for _ in 0..8 {
            let cache = cache.clone();
            let loads = loads.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                cache
                    .get_or_load(key("k"), || async move {
                        loads.fetch_add(1, Ordering::SeqCst);
                        // Hold the load open so the others pile onto the flight.
                        tokio::time::sleep(Duration::from_millis(30)).await;
                        Ok(response(1))
                    })
                    .await
                    .unwrap();
            }));
        }
        for h in handles {
            h.await.unwrap();
        }

        assert_eq!(loads.load(Ordering::SeqCst), 1, "exactly one backing load");
        let (_, misses, coalesced) = cache.stats().snapshot();
        assert_eq!(misses, 1, "one leader");
        assert_eq!(coalesced, 7, "seven followers joined the flight");
    }

    #[tokio::test]
    async fn errors_are_not_cached() {
        let cache = QueryCache::new(Duration::from_secs(60), 100);
        let first = cache
            .get_or_load(key("k"), || async {
                Err(Error::Invalid {
                    message: "boom".into(),
                })
            })
            .await;
        assert!(first.is_err());
        // A subsequent success must run rather than serve the cached error.
        let second = cache
            .get_or_load(key("k"), || async { Ok(response(7)) })
            .await
            .unwrap();
        assert_eq!(second.stats.row_count, 7);
    }
}
