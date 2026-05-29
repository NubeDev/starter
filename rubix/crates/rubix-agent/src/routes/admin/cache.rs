//! `GET /api/v1/admin/cache/specs` — per-spec cache hit/miss snapshot.
//!
//! Read-only projection over [`starter_cache::CacheLayer`]'s per-spec
//! stats joined with the registered [`starter_ext_server::KindCacheRegistry`].
//! The join is important: a spec registered but never touched needs
//! to appear with zero counters so the operator can answer "is this
//! kind ever called?".
//!
//! Output shape:
//!
//! ```json
//! {
//!   "specs": [
//!     {
//!       "spec_id": "com.nubeio.rubixos::com.nubeio.rubixos.warehouse_query",
//!       "extension": "com.nubeio.rubixos",
//!       "contribute_id": "com.nubeio.rubixos.warehouse_query",
//!       "config": {
//!         "ttl_seconds": 60,
//!         "scope": "user",
//!         "invalidate_on_tables": ["com_nubeio_rubixos__histories"]
//!       },
//!       "hits": 42,
//!       "misses": 7,
//!       "hit_ratio": 0.857,
//!       "load_latency": { ... }
//!     }
//!   ]
//! }
//! ```
//!
//! When the cache layer is not wired (developer rigs that disable
//! extensions), the endpoint returns `{ "specs": [] }`.

use axum::extract::{Path, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::Json;
use serde::{Deserialize, Serialize};
use starter_cache::{CacheScope, CacheSpec, PerSpecSnapshot, TimeSeriesBlock};
use std::collections::BTreeMap;

use crate::admin::AdminState;
use crate::routes::{RouteMeta, RouteRegistrar};

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new()
        .mount(
            Method::GET,
            "/api/v1/admin/cache/specs",
            get(list).with_state(state.clone()),
            RouteMeta::new()
                .describe("List per-spec config + hit/miss counters for the opt-in cache.")
                .tag("admin"),
        )
        .mount(
            Method::POST,
            "/api/v1/admin/cache/invalidate",
            post(invalidate).with_state(state.clone()),
            RouteMeta::new()
                .describe(
                    "Fire `invalidate_tags(tags)` against the opt-in cache. \
                     Every cached entry whose stored snapshot depended on any of \
                     the named tags becomes a miss on next read. The body shape \
                     is `{ \"tags\": [\"table:foo\", \"table:bar\"] }`.",
                )
                .tag("admin"),
        )
        .mount(
            Method::POST,
            "/api/v1/admin/cache/invalidate_all",
            post(invalidate_all).with_state(state.clone()),
            RouteMeta::new()
                .describe(
                    "Drop every cached entry across every tenant. Last-resort \
                     escape hatch — prefer `/invalidate` (tag-scoped) or \
                     `/tenants/{tenant}` (tenant-scoped) when possible. \
                     Per-spec counters and tag tokens survive so the operator \
                     can watch hit rate recover from a known baseline.",
                )
                .tag("admin"),
        )
        .mount(
            Method::DELETE,
            "/api/v1/admin/cache/tenants/{tenant}",
            delete(evict_tenant).with_state(state),
            RouteMeta::new()
                .describe(
                    "Drop every cached entry belonging to `{tenant}`. \
                     Use when a tenant is disabled or deleted and the operator \
                     wants to reclaim cache memory immediately rather than \
                     waiting for the per-tenant entries to TTL out.",
                )
                .tag("admin"),
        )
}

#[derive(Serialize)]
struct SpecRow {
    spec_id: String,
    /// `extension` and `contribute_id` are the split form of
    /// `spec_id`. Both included so tooling doesn't have to re-parse
    /// the delimiter.
    extension: String,
    contribute_id: String,
    /// `null` when the registry hasn't been wired into the admin
    /// state (or, in the joined-stats path, when a counter exists
    /// without a matching registered spec — should not happen in
    /// practice but the wire shape allows it for robustness).
    config: Option<SpecConfig>,
    hits: u64,
    misses: u64,
    hit_ratio: f64,
    load_latency: LoadLatencyRow,
}

#[derive(Serialize)]
struct SpecConfig {
    ttl_seconds: u64,
    scope: &'static str,
    invalidate_on_tables: Vec<String>,
    /// v1: SWR window in seconds. `0` when SWR is disabled.
    stale_while_revalidate_seconds: u64,
    /// v1: cap on the empty-result cache window.
    empty_ttl_seconds: u64,
    /// v1: whether empty results are cached at all.
    cache_empty: bool,
    /// v1: write-path event tags this spec is subscribed to.
    invalidate_on_events: Vec<String>,
    /// v1: structured bucket subscription (`{table, granularity}`).
    /// `null` when the spec declares no bucket subscription.
    invalidate_on_buckets: Option<BucketSpecRow>,
    /// v2: opt-in time-series block (`null` when absent).
    time_series: Option<TimeSeriesRow>,
    /// v2: two-layer cache `inner_scope` (`null` when absent).
    inner_scope: Option<&'static str>,
}

#[derive(Serialize)]
struct TimeSeriesRow {
    time_param: String,
    range_param: String,
    bucket: String,
    tail_ttl: String,
    body_ttl: String,
    align_to: String,
}

impl TimeSeriesRow {
    fn from_block(b: &TimeSeriesBlock) -> Self {
        Self {
            time_param: b.time_param.clone(),
            range_param: b.range_param.clone(),
            bucket: b.bucket.clone(),
            tail_ttl: b.tail_ttl.clone(),
            body_ttl: b.body_ttl.clone(),
            align_to: b.align_to.clone(),
        }
    }
}

#[derive(Serialize)]
struct BucketSpecRow {
    table: String,
    granularity: String,
}

impl SpecConfig {
    fn from_spec(spec: &CacheSpec) -> Self {
        Self {
            ttl_seconds: spec.ttl.as_secs(),
            scope: scope_str(spec.scope),
            invalidate_on_tables: spec.invalidate_on.tables.clone(),
            stale_while_revalidate_seconds: spec.stale_while_revalidate.as_secs(),
            empty_ttl_seconds: spec.empty_ttl.as_secs(),
            cache_empty: spec.cache_empty,
            invalidate_on_events: spec.invalidate_on.events.clone(),
            invalidate_on_buckets: spec.invalidate_on.buckets.as_ref().map(|b| BucketSpecRow {
                table: b.table.clone(),
                granularity: b.granularity.clone(),
            }),
            time_series: spec.time_series.as_ref().map(TimeSeriesRow::from_block),
            inner_scope: spec.inner_scope.map(scope_str),
        }
    }
}

fn scope_str(s: CacheScope) -> &'static str {
    match s {
        CacheScope::Global => "global",
        CacheScope::Tenant => "tenant",
        CacheScope::User => "user",
    }
}

#[derive(Serialize, Default)]
struct LoadLatencyRow {
    le_10ms: u64,
    le_100ms: u64,
    le_1s: u64,
    le_10s: u64,
    gt_10s: u64,
    count: u64,
    sum_nanos: u64,
    mean_ms: f64,
}

impl LoadLatencyRow {
    fn from_snapshot(s: &starter_cache::LoadLatencySnapshot) -> Self {
        let mean_ms = if s.count == 0 {
            0.0
        } else {
            (s.sum_nanos as f64) / (s.count as f64) / 1_000_000.0
        };
        Self {
            le_10ms: s.le_10ms,
            le_100ms: s.le_100ms,
            le_1s: s.le_1s,
            le_10s: s.le_10s,
            gt_10s: s.gt_10s,
            count: s.count,
            sum_nanos: s.sum_nanos,
            mean_ms,
        }
    }
}

#[derive(Serialize)]
struct ListBody {
    specs: Vec<SpecRow>,
}

fn split_spec_id(spec_id: &str) -> (String, String) {
    // Spec id shape: `"{extension}::{contribute_id}"`. The split
    // is purely cosmetic for the wire — counters work either way.
    match spec_id.split_once("::") {
        Some((ext, cid)) => (ext.to_string(), cid.to_string()),
        None => (String::new(), spec_id.to_string()),
    }
}

async fn list(State(state): State<AdminState>) -> Response {
    let Some(layer) = state.cache_layer.as_ref() else {
        return Json(ListBody { specs: Vec::new() }).into_response();
    };

    // Join: every registered spec + every spec that has been
    // touched (the touched set will normally be a subset of
    // registered, but guard against drift).
    let mut by_id: BTreeMap<String, SpecRow> = BTreeMap::new();

    if let Some(reg) = state.cache_registry.as_ref() {
        for (ext, contribute_id, spec) in reg.iter() {
            let spec_id = format!("{}::{}", ext.as_str(), contribute_id);
            by_id.insert(
                spec_id.clone(),
                SpecRow {
                    spec_id,
                    extension: ext.as_str().to_string(),
                    contribute_id: contribute_id.to_string(),
                    config: Some(SpecConfig::from_spec(spec)),
                    hits: 0,
                    misses: 0,
                    hit_ratio: 0.0,
                    load_latency: LoadLatencyRow::default(),
                },
            );
        }
    }

    for snap in layer.per_spec_snapshot() {
        let PerSpecSnapshot {
            spec_id,
            hits,
            misses,
            hit_ratio,
            load_latency,
        } = snap;
        let (extension, contribute_id) = split_spec_id(&spec_id);
        let row = by_id.entry(spec_id.clone()).or_insert_with(|| SpecRow {
            spec_id,
            extension,
            contribute_id,
            config: None,
            hits: 0,
            misses: 0,
            hit_ratio: 0.0,
            load_latency: LoadLatencyRow::default(),
        });
        row.hits = hits;
        row.misses = misses;
        row.hit_ratio = hit_ratio;
        row.load_latency = LoadLatencyRow::from_snapshot(&load_latency);
    }

    Json(ListBody {
        specs: by_id.into_values().collect(),
    })
    .into_response()
}

/// Body of `POST /api/v1/admin/cache/invalidate`.
#[derive(Debug, Deserialize)]
struct InvalidateBody {
    /// Tags to fire. Each entry must be a fully-qualified tag string
    /// (e.g. `"table:com_nubeio_rubixos__histories"`). Empty array is
    /// accepted and is a no-op — easier on tooling than a 400.
    tags: Vec<String>,
}

/// Response shape.
#[derive(Debug, Serialize)]
struct InvalidateResponse {
    /// Number of tags actually fired (equal to `body.tags.len()` after
    /// dedup-by-position; the layer itself dedupes on the read path).
    invalidated: usize,
}

async fn invalidate(State(state): State<AdminState>, Json(body): Json<InvalidateBody>) -> Response {
    let Some(layer) = state.cache_layer.as_ref() else {
        // No cache wired — the request was a no-op from the layer's
        // perspective. 503 makes the wire shape unambiguous: the
        // operator's request did not take effect, do not assume it
        // did.
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "service_unavailable",
                "message": "opt-in cache is not wired on this host",
            })),
        )
            .into_response();
    };
    let n = body.tags.len();
    layer.invalidator().invalidate_tags(&body.tags).await;
    Json(InvalidateResponse { invalidated: n }).into_response()
}

/// Response shape for the global invalidate endpoint.
#[derive(Debug, Serialize)]
struct InvalidateAllResponse {
    /// Approximate total entries dropped across every tenant cache.
    entries_dropped: u64,
}

async fn invalidate_all(State(state): State<AdminState>) -> Response {
    let Some(layer) = state.cache_layer.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "service_unavailable",
                "message": "opt-in cache is not wired on this host",
            })),
        )
            .into_response();
    };
    let dropped = layer.invalidate_all().await;
    Json(InvalidateAllResponse {
        entries_dropped: dropped,
    })
    .into_response()
}

/// Response shape for the per-tenant eviction endpoint.
#[derive(Debug, Serialize)]
struct EvictTenantResponse {
    /// The tenant id evicted (echoed back so a caller using a
    /// wildcard URL pattern can confirm which one fired).
    tenant: String,
    /// Approximate number of entries dropped. moka's `entry_count`
    /// is eventually consistent, so this is the count at eviction
    /// time, not a strict guarantee.
    entries_dropped: u64,
}

async fn evict_tenant(State(state): State<AdminState>, Path(tenant): Path<String>) -> Response {
    let Some(layer) = state.cache_layer.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "service_unavailable",
                "message": "opt-in cache is not wired on this host",
            })),
        )
            .into_response();
    };
    let dropped = layer.evict_tenant(&tenant).await;
    Json(EvictTenantResponse {
        tenant,
        entries_dropped: dropped,
    })
    .into_response()
}
