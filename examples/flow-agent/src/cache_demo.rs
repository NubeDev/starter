//! `/api/cache-demo/*` — standalone demo of `starter-cache`.
//!
//! Generates a synthetic per-minute time-series on every cache miss
//! and serves it back from a `MokaCache` on subsequent hits. Cold
//! cost is **real work** (signal generation + bucket aggregation +
//! JSON serialisation of ~5k points); warm cost is just the body
//! serialisation. No sleeps.
//!
//! Routes:
//!
//! - `GET  /api/cache-demo/series?bucket=1m|5m|15m&points=N`
//! - `GET  /api/cache-demo/stats`
//! - `POST /api/cache-demo/clear`

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use starter_cache::backends::moka::MokaCache;
use starter_cache::Cache;

/// Default number of chart buckets (and thus rendered points).
const DEFAULT_POINTS: usize = 5_000;
/// Hard upper bound on chart buckets.
const MAX_POINTS: usize = 50_000;
/// Raw samples per chart bucket per series. Bumps cold-load work
/// into a visible range so the cache benefit is honest
/// (e.g. 5k buckets × 200 × 4 series = 4M raw samples, each then
/// smoothed with a 32-wide centred moving average).
const RAW_MULTIPLIER: usize = 200;
/// Window size for the moving-average pass over each raw signal.
const SMOOTHING_WINDOW: usize = 32;

/// Definitions for the metric series returned by `/series`.
/// `(name, unit, color, base, amplitude, noise, seed)`.
const SERIES_DEFS: &[(&str, &str, &str, f64, f64, f64, u32)] = &[
    ("requests_per_sec", "req/s", "#3b82f6", 1200.0, 320.0, 80.0, 0x9e37_79b9),
    ("p50_latency_ms", "ms", "#22c55e", 42.0, 8.0, 3.0, 0x517c_c1b7),
    ("p99_latency_ms", "ms", "#ef4444", 180.0, 60.0, 25.0, 0x85eb_ca77),
    ("error_rate_pct", "%", "#f59e0b", 0.6, 0.45, 0.2, 0x27d4_eb2f),
];

/// One bucket in a returned series.
#[derive(Debug, Clone, Serialize)]
pub struct Sample {
    /// RFC3339 timestamp (UTC).
    pub t: String,
    /// Average value across the bucket.
    pub avg: f64,
    /// Min value across the bucket.
    pub min: f64,
    /// Max value across the bucket.
    pub max: f64,
}

/// One named metric series in the response.
#[derive(Debug, Clone, Serialize)]
pub struct MetricSeries {
    pub name: String,
    pub unit: String,
    pub color: String,
    pub points: Vec<Sample>,
}

/// Server response for `GET /series`.
#[derive(Debug, Clone, Serialize)]
pub struct SeriesResponse {
    pub series: Vec<MetricSeries>,
    pub raw_points: usize,
    pub bucket_minutes: u32,
    pub generated_in_ms: u64,
    pub from_cache: bool,
}

/// Query params for `GET /series`.
#[derive(Debug, Deserialize, Default)]
pub struct SeriesQuery {
    /// Bucket size string: `1m`, `5m`, `15m`, `60m`. Default `5m`.
    pub bucket: Option<String>,
    /// Number of raw samples to synthesise. Default
    /// [`DEFAULT_POINTS`], capped at [`MAX_POINTS`].
    pub points: Option<usize>,
}

/// Snapshot shape for `GET /stats`.
#[derive(Debug, Serialize)]
pub struct CacheStatsView {
    pub hits: u64,
    pub misses: u64,
    pub hit_ratio: f64,
    pub entries: u64,
    pub last_cold_load_ms: u64,
    pub last_warm_load_ms: u64,
    pub backend: &'static str,
}

/// Shared state. Cheap to clone.
#[derive(Clone)]
pub struct CacheDemoState {
    cache: MokaCache<String, Arc<SeriesResponse>>,
    last_cold_ms: Arc<AtomicU64>,
    last_warm_ms: Arc<AtomicU64>,
}

impl CacheDemoState {
    pub fn new() -> Self {
        Self {
            cache: MokaCache::builder().max_capacity(64).build(),
            last_cold_ms: Arc::new(AtomicU64::new(0)),
            last_warm_ms: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Default for CacheDemoState {
    fn default() -> Self {
        Self::new()
    }
}

/// Mount the demo on a fresh router. Generic over the host's outer
/// state `S` so it merges into `starter-server`'s `Router<AppState>`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = CacheDemoState::new();
    Router::new()
        .route("/api/cache-demo/series", get(get_series))
        .route("/api/cache-demo/stats", get(get_stats))
        .route("/api/cache-demo/clear", post(clear))
        .with_state(state)
}

async fn get_series(
    State(s): State<CacheDemoState>,
    Query(q): Query<SeriesQuery>,
) -> Json<SeriesResponse> {
    let bucket_minutes = parse_bucket(q.bucket.as_deref()).unwrap_or(5);
    let points = q.points.unwrap_or(DEFAULT_POINTS).min(MAX_POINTS).max(1);
    let key = format!("series|bucket={bucket_minutes}m|points={points}");

    let started = Instant::now();

    // In-loader flag so we can attribute the call to hit vs miss
    // WITHOUT a pre-probe (a pre-probe would double-count). The
    // loader runs at most once per key (moka single-flight).
    let miss = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let miss_writer = Arc::clone(&miss);

    let value: Arc<SeriesResponse> = s
        .cache
        .get_or_insert_with::<_, _, std::convert::Infallible>(key.clone(), move || async move {
            miss_writer.store(true, Ordering::Relaxed);
            let t0 = Instant::now();
            let raw_count = points * RAW_MULTIPLIER;
            let stride = (raw_count / points).max(1);
            let mut series = Vec::with_capacity(SERIES_DEFS.len());
            let mut total_raw = 0usize;
            for &(name, unit, color, base, amp, noise, seed) in SERIES_DEFS {
                let raw = synth_signal(raw_count, base, amp, noise, seed);
                let smoothed = moving_average(&raw, SMOOTHING_WINDOW);
                let bucketed = aggregate(&smoothed, stride, bucket_minutes as usize);
                total_raw += raw_count;
                series.push(MetricSeries {
                    name: name.to_owned(),
                    unit: unit.to_owned(),
                    color: color.to_owned(),
                    points: bucketed,
                });
            }
            Ok(Arc::new(SeriesResponse {
                series,
                raw_points: total_raw,
                bucket_minutes,
                generated_in_ms: t0.elapsed().as_millis() as u64,
                from_cache: false,
            }))
        })
        .await
        .expect("infallible loader");

    let elapsed = started.elapsed().as_millis() as u64;
    let was_miss = miss.load(Ordering::Relaxed);
    if was_miss {
        s.last_cold_ms.store(elapsed, Ordering::Relaxed);
    } else {
        s.last_warm_ms.store(elapsed, Ordering::Relaxed);
    }

    let mut out = (*value).clone();
    out.from_cache = !was_miss;
    Json(out)
}

async fn get_stats(State(s): State<CacheDemoState>) -> Json<CacheStatsView> {
    Json(snapshot(&s))
}

async fn clear(State(s): State<CacheDemoState>) -> Json<CacheStatsView> {
    s.cache.invalidate_all().await;
    s.cache.stats().reset();
    s.last_cold_ms.store(0, Ordering::Relaxed);
    s.last_warm_ms.store(0, Ordering::Relaxed);
    Json(snapshot(&s))
}

fn snapshot(s: &CacheDemoState) -> CacheStatsView {
    let st = s.cache.stats();
    CacheStatsView {
        hits: st.hits(),
        misses: st.misses(),
        hit_ratio: st.hit_ratio(),
        entries: s.cache.entry_count(),
        last_cold_load_ms: s.last_cold_ms.load(Ordering::Relaxed),
        last_warm_load_ms: s.last_warm_ms.load(Ordering::Relaxed),
        backend: "moka",
    }
}

fn parse_bucket(s: Option<&str>) -> Option<u32> {
    let s = s?.trim().to_ascii_lowercase();
    let n: u32 = s.strip_suffix('m')?.parse().ok()?;
    // Constrain to a sensible set so misuse can't blow up the chart.
    matches!(n, 1 | 5 | 15 | 30 | 60).then_some(n)
}

/// Synthesise a deterministic time-series given a base level,
/// amplitude, noise scale, and seed. Same inputs always yield the
/// same output. No `rand` dep, no clock-anchoring.
fn synth_signal(n: usize, base: f64, amp: f64, noise: f64, seed: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = i as f64;
        // Diurnal-ish base wave.
        let wave = base + amp * (x / 180.0).sin();
        // Slow drift specific to this seed.
        let drift = (amp * 0.25) * ((x + seed as f64 * 0.001) / 1500.0).cos();
        // Cheap deterministic noise via a hashed wave keyed by seed.
        let mixed = (i as u32).wrapping_mul(2_654_435_761).wrapping_add(seed);
        let noise_v = (((mixed & 0xff) as f64 / 255.0) - 0.5) * 2.0 * noise;
        // Occasional spike every ~400 samples, sized by amp.
        let spike = if i % 400 == 0 && i != 0 { amp * 0.6 } else { 0.0 };
        out.push(wave + drift + noise_v + spike);
    }
    out
}

/// Aggregate `raw` into per-bucket min/avg/max samples by chunking
/// `raw` into windows of `chunk_size` and spacing the timestamps
/// `bucket_minutes` apart, anchored at a fixed historical instant.
fn aggregate(raw: &[f64], chunk_size: usize, bucket_minutes: usize) -> Vec<Sample> {
    if raw.is_empty() || chunk_size == 0 || bucket_minutes == 0 {
        return Vec::new();
    }
    let anchor =
        chrono::DateTime::parse_from_rfc3339("2026-05-01T00:00:00Z").expect("static rfc3339");
    let mut out = Vec::with_capacity(raw.len() / chunk_size + 1);
    for (idx, chunk) in raw.chunks(chunk_size).enumerate() {
        let (mut min, mut max, mut sum) = (chunk[0], chunk[0], 0.0);
        for v in chunk {
            if *v < min {
                min = *v;
            }
            if *v > max {
                max = *v;
            }
            sum += *v;
        }
        let avg = sum / chunk.len() as f64;
        let t = anchor + chrono::Duration::minutes((idx * bucket_minutes) as i64);
        out.push(Sample {
            t: t.to_rfc3339(),
            avg: round2(avg),
            min: round2(min),
            max: round2(max),
        });
    }
    out
}

/// Simple centred moving-average smoother over `raw`. O(n*window)
/// — deliberately not a rolling-sum optimisation so the cold load
/// actually has weight to it.
fn moving_average(raw: &[f64], window: usize) -> Vec<f64> {
    if raw.is_empty() || window <= 1 {
        return raw.to_vec();
    }
    let half = window / 2;
    let mut out = Vec::with_capacity(raw.len());
    for i in 0..raw.len() {
        let start = i.saturating_sub(half);
        let end = (i + half + 1).min(raw.len());
        let slice = &raw[start..end];
        let sum: f64 = slice.iter().copied().sum();
        out.push(sum / slice.len() as f64);
    }
    out
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_is_deterministic() {
        let a = synth_signal(10, 50.0, 15.0, 4.0, 0x9e37_79b9);
        let b = synth_signal(10, 50.0, 15.0, 4.0, 0x9e37_79b9);
        assert_eq!(a, b);
    }

    #[test]
    fn aggregate_buckets_sized_correctly() {
        let raw = synth_signal(100, 50.0, 15.0, 4.0, 0x9e37_79b9);
        let out = aggregate(&raw, 5, 5);
        assert_eq!(out.len(), 20);
        for s in &out {
            assert!(s.min <= s.avg && s.avg <= s.max);
        }
    }

    #[test]
    fn parse_bucket_accepts_known_values() {
        for s in ["1m", "5m", "15m", "30m", "60m"] {
            assert!(parse_bucket(Some(s)).is_some());
        }
        assert!(parse_bucket(Some("7m")).is_none());
        assert!(parse_bucket(Some("foo")).is_none());
    }
}
