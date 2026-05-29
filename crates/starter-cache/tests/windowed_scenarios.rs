//! v2 — windowed / two-layer scenarios.
//!
//! Maps the proposal's stage-2 acceptance scenarios onto the cache
//! layer adapter:
//!
//! - bucket decomposition into per-bucket cache entries
//! - body_ttl vs tail_ttl applied to the right buckets
//! - delta-fetch: a cached 7d range extended to 90d only re-fetches
//!   the missing prefix buckets
//! - two-layer hit (outer user miss + inner tenant hit)
//! - bucket-level invalidation hits one bucket only

use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use starter_cache::{
    CacheLayer, CacheScope, CacheSpec, CallerScope, InMemoryInvalidator, Invalidator, LayerConfig,
    MockClock, TimeSeriesBlock,
};
use starter_windowed::{Bucket, FetchError, RowSet, WindowedFetcher};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

fn ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// Counts per-bucket fetch calls so tests can assert exact cache
/// reuse.
struct RecordingFetcher {
    calls: Arc<AtomicU32>,
    seen: Arc<Mutex<Vec<DateTime<Utc>>>>,
}

impl RecordingFetcher {
    fn new() -> Self {
        Self {
            calls: Arc::new(AtomicU32::new(0)),
            seen: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl WindowedFetcher<RowSet> for RecordingFetcher {
    async fn fetch_bucket(&self, bucket: Bucket) -> Result<RowSet, FetchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.seen.lock().unwrap().push(bucket.start);
        // One row per bucket — payload value carries the bucket key
        // so stitching is observable.
        let v = serde_json::json!({ "bucket": bucket.start.to_rfc3339() });
        Ok(RowSet::new(vec![v]))
    }
}

fn windowed_spec_hourly() -> CacheSpec {
    CacheSpec::ttl(std::time::Duration::from_secs(60))
        .scope(CacheScope::Tenant)
        .invalidate_on_table("histories")
        .time_series(TimeSeriesBlock {
            time_param: "to".into(),
            range_param: "from".into(),
            bucket: "1h".into(),
            tail_ttl: "30s".into(),
            body_ttl: "24h".into(),
            align_to: "utc".into(),
        })
}

#[tokio::test]
async fn bucket_decomposition_and_reuse() {
    let layer = CacheLayer::new(LayerConfig::default());
    let spec = windowed_spec_hourly();
    let caller = CallerScope::new("tA", "uX");
    let fetcher = RecordingFetcher::new();

    // 7h window → 7+1 = 8 buckets (from..to snapped to hour).
    let to = ts("2026-05-29T07:30:00Z");
    let from = to - ChronoDuration::hours(7);

    let out = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    assert_eq!(out.rows().len(), 8, "decompose 7h+tail = 8 buckets");
    assert_eq!(fetcher.calls.load(Ordering::SeqCst), 8);

    // Repeat: every bucket hits cache.
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    assert_eq!(
        fetcher.calls.load(Ordering::SeqCst),
        8,
        "second call must be all-hit"
    );
}

#[tokio::test]
async fn delta_fetch_extends_cached_7d_to_90d() {
    let layer = CacheLayer::new(LayerConfig::default());
    let spec = windowed_spec_hourly();
    let caller = CallerScope::new("tA", "uX");
    let fetcher = RecordingFetcher::new();

    let to = ts("2026-05-29T00:00:00Z");
    let from_7d = to - ChronoDuration::hours(24 * 7);
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from_7d, to, &fetcher)
        .await
        .unwrap();
    let after_7d = fetcher.calls.load(Ordering::SeqCst);
    assert!(after_7d > 0);

    // Extend to 90d — only the missing prefix should fetch.
    let from_90d = to - ChronoDuration::hours(24 * 90);
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from_90d, to, &fetcher)
        .await
        .unwrap();
    let after_90d = fetcher.calls.load(Ordering::SeqCst);
    let new_fetches = after_90d - after_7d;
    // 90d - 7d = 83d of new buckets (hourly).
    assert_eq!(
        new_fetches,
        83 * 24,
        "extending 7d→90d must re-fetch only the 83d prefix"
    );
}

#[tokio::test]
async fn body_vs_tail_ttl_respected() {
    let clock = MockClock::new();
    let layer = CacheLayer::with_parts(
        LayerConfig::default(),
        Arc::new(clock.clone()),
        Arc::new(InMemoryInvalidator::new()),
    );
    let spec = windowed_spec_hourly();
    let caller = CallerScope::new("tA", "uX");
    let fetcher = RecordingFetcher::new();

    let to = Utc.with_ymd_and_hms(2026, 5, 29, 3, 30, 0).unwrap();
    let from = to - ChronoDuration::hours(2);
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    let initial = fetcher.calls.load(Ordering::SeqCst);
    // 2h+tail = 3 buckets.
    assert_eq!(initial, 3);

    // Advance past tail_ttl (30s) but well inside body_ttl (24h).
    clock.advance(std::time::Duration::from_secs(45));
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    // Only the tail bucket should have re-fetched: 1 extra call.
    assert_eq!(
        fetcher.calls.load(Ordering::SeqCst) - initial,
        1,
        "only the tail bucket must be refetched after tail_ttl"
    );
}

#[tokio::test]
async fn bucket_level_invalidation_hits_one_bucket_only() {
    let invalidator = Arc::new(InMemoryInvalidator::new());
    let layer = CacheLayer::with_parts(
        LayerConfig::default(),
        Arc::new(starter_cache::SystemClock),
        invalidator.clone(),
    );
    let spec = windowed_spec_hourly();
    let caller = CallerScope::new("tA", "uX");
    let fetcher = RecordingFetcher::new();

    let to = ts("2026-05-29T07:00:00Z");
    let from = to - ChronoDuration::hours(5);
    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    let after_first = fetcher.calls.load(Ordering::SeqCst);
    assert_eq!(after_first, 6);

    // Fire the bucket-level tag for one specific bucket.
    let bucket_to_blow = ts("2026-05-29T04:00:00Z").to_rfc3339();
    invalidator
        .invalidate_tags(&[format!("bucket:histories:{bucket_to_blow}")])
        .await;

    let _ = layer
        .get_or_load_windowed::<RowSet>(&spec, None, &caller, "wq", from, to, &fetcher)
        .await
        .unwrap();
    let after_second = fetcher.calls.load(Ordering::SeqCst);
    assert_eq!(
        after_second - after_first,
        1,
        "bucket-level invalidation must drop exactly one bucket"
    );
}

#[tokio::test]
async fn two_layer_outer_miss_inner_hit() {
    let layer = CacheLayer::new(LayerConfig::default());
    let spec = CacheSpec::ttl(std::time::Duration::from_secs(60))
        .scope(CacheScope::User)
        .inner_scope(CacheScope::Tenant);

    let caller_a = CallerScope::new("tA", "uA");
    let caller_b = CallerScope::new("tA", "uB");
    let canonical_calls = Arc::new(AtomicU32::new(0));
    let render_calls = Arc::new(AtomicU32::new(0));

    let bytes = |s: &str| -> starter_cache::Bytes { Arc::new(s.as_bytes().to_vec()) };
    let canonical_calls_a = canonical_calls.clone();
    let render_calls_a = render_calls.clone();
    let _v = layer
        .get_or_load_two_layer::<_, _, _, _, std::convert::Infallible>(
            &spec,
            None,
            &caller_a,
            "k",
            move || async move {
                canonical_calls_a.fetch_add(1, Ordering::SeqCst);
                Ok(bytes("CANON"))
            },
            move |b| async move {
                render_calls_a.fetch_add(1, Ordering::SeqCst);
                let mut v = (*b).clone();
                v.extend_from_slice(b":uA");
                Ok(Arc::new(v))
            },
        )
        .await
        .unwrap();

    let canonical_calls_b = canonical_calls.clone();
    let render_calls_b = render_calls.clone();
    let v_b = layer
        .get_or_load_two_layer::<_, _, _, _, std::convert::Infallible>(
            &spec,
            None,
            &caller_b,
            "k",
            move || async move {
                canonical_calls_b.fetch_add(1, Ordering::SeqCst);
                Ok(bytes("CANON"))
            },
            move |b| async move {
                render_calls_b.fetch_add(1, Ordering::SeqCst);
                let mut v = (*b).clone();
                v.extend_from_slice(b":uB");
                Ok(Arc::new(v))
            },
        )
        .await
        .unwrap();

    assert_eq!(
        canonical_calls.load(Ordering::SeqCst),
        1,
        "inner (tenant) cache must absorb the second user's DB hit"
    );
    assert_eq!(
        render_calls.load(Ordering::SeqCst),
        2,
        "render runs once per user (outer is user-scope)"
    );
    assert_eq!(&*v_b, b"CANON:uB");
}
