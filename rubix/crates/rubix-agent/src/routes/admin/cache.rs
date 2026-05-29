//! `GET /api/v1/admin/cache/specs` — per-spec cache hit/miss snapshot.
//!
//! Read-only projection over [`starter_cache::CacheLayer`]'s per-spec
//! stats. The v0 caching cut needs this to answer "is the
//! `usage_bucketed` canary actually paying off?" without a separate
//! metrics pipeline. Output shape:
//!
//! ```json
//! {
//!   "specs": [
//!     {
//!       "spec_id": "com.nubeio.rubixos::usage_bucketed",
//!       "hits": 42,
//!       "misses": 7,
//!       "hit_ratio": 0.857
//!     }
//!   ]
//! }
//! ```
//!
//! When the cache layer is not wired (developer rigs that disable
//! extensions), the endpoint returns `{ "specs": [] }` — same shape,
//! easier for tooling than a 404.

use axum::extract::State;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Json;
use serde::Serialize;

use crate::admin::AdminState;
use crate::routes::{RouteMeta, RouteRegistrar};

pub(super) fn registrar(state: AdminState) -> RouteRegistrar {
    RouteRegistrar::new().mount(
        Method::GET,
        "/api/v1/admin/cache/specs",
        get(list).with_state(state),
        RouteMeta::new()
            .describe("List per-spec hit/miss counters for the opt-in cache.")
            .tag("admin"),
    )
}

#[derive(Serialize)]
struct SpecRow {
    spec_id: String,
    hits: u64,
    misses: u64,
    hit_ratio: f64,
}

#[derive(Serialize)]
struct ListBody {
    specs: Vec<SpecRow>,
}

async fn list(State(state): State<AdminState>) -> Response {
    let specs = match state.cache_layer.as_ref() {
        Some(layer) => layer
            .per_spec_snapshot()
            .into_iter()
            .map(|s| SpecRow {
                spec_id: s.spec_id,
                hits: s.hits,
                misses: s.misses,
                hit_ratio: s.hit_ratio,
            })
            .collect(),
        None => Vec::new(),
    };
    Json(ListBody { specs }).into_response()
}
