//! REST + SSE surface per the SCOPE "REST and SSE surface" table.
//!
//! All handlers forward into [`crate::nodes::runtime::WarehouseRuntime`];
//! the warehouse never has two implementations of the same node body.

mod sse;
mod status;

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use starter_tags::TagQuery;
use std::str::FromStr;

use crate::catalog::mart_spec::MartSpec;
use crate::ddl::sandbox::SandboxSpec;
use crate::nodes::runtime::{RuntimeError, WarehouseRuntime};

pub use sse::sse_router;
pub use status::WarehouseStatusBody;

/// Mount the full warehouse REST surface under `/api`.
///
/// The ClickHouse explorer sub-router (PR 1 of the explorer plan)
/// is gated by the `warehouse.read` permission. The existing
/// `/api/warehouse/*` handlers in this module are intentionally
/// **not** gated here — the codebase did not previously apply
/// `starter-authz` to them, and changing their gating is out of
/// scope for the explorer PR. Bring-up callers that want
/// `warehouse.read`/`warehouse.write` enforced on the existing
/// handlers can layer it on the merged router themselves.
pub fn router(rt: Arc<WarehouseRuntime>) -> Router {
    let explorer = starter_authz::with_permission(
        crate::explorer::routes(rt.ch.clone()),
        "warehouse",
        "read",
    );
    Router::new()
        .route("/api/marts", post(create_mart))
        .route("/api/marts/{name}", delete(drop_mart))
        .route("/api/marts/{name}/promote", post(promote_mart))
        .route("/api/marts/{name}/data", get(read_mart))
        .route("/api/sandboxes", post(create_sandbox))
        .route("/api/sandboxes/{name}", delete(drop_sandbox))
        .route("/api/warehouse/gc", post(run_gc))
        .route("/api/warehouse/status", get(status::warehouse_status))
        .route("/api/warehouse/audit", post(run_audit))
        .merge(sse::sse_router())
        .with_state(rt)
        .merge(explorer)
}

async fn create_mart(
    State(rt): State<Arc<WarehouseRuntime>>,
    Json(spec): Json<MartSpec>,
) -> impl IntoResponse {
    match rt.mart_define(spec).await {
        Ok(r) => (StatusCode::CREATED, Json(serde_json::to_value(&r).unwrap())).into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

async fn drop_mart(
    State(rt): State<Arc<WarehouseRuntime>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match rt.mart_drop(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

#[derive(Deserialize)]
struct PromoteQuery {
    approved_by: Option<String>,
    ext_manifest_hash: Option<String>,
}

async fn promote_mart(
    State(rt): State<Arc<WarehouseRuntime>>,
    Path(name): Path<String>,
    Query(q): Query<PromoteQuery>,
) -> impl IntoResponse {
    let by = q.approved_by.unwrap_or_else(|| "user:admin".into());
    match rt
        .mart_promote(&name, &by, q.ext_manifest_hash.as_deref())
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

#[derive(Deserialize)]
struct ReadQuery {
    filter: String,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    #[serde(default)]
    hide_unknown: bool,
    #[serde(default = "default_max_buckets")]
    max_buckets: u32,
}

fn default_max_buckets() -> u32 {
    crate::nodes::mart_read::DEFAULT_MAX_BUCKETS
}

async fn read_mart(
    State(rt): State<Arc<WarehouseRuntime>>,
    Path(name): Path<String>,
    Query(q): Query<ReadQuery>,
) -> impl IntoResponse {
    let filter = match TagQuery::from_str(&q.filter) {
        Ok(f) => f,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "tag_query_parse", "message": e.to_string()})),
            )
                .into_response()
        }
    };
    if q.max_buckets > 100_000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "max_buckets_too_large", "max": 100_000})),
        )
            .into_response();
    }
    match rt
        .mart_read(&name, filter, q.from, q.to, q.hide_unknown, q.max_buckets)
        .await
    {
        Ok(r) => (StatusCode::OK, Json(serde_json::to_value(&r).unwrap())).into_response(),
        Err(RuntimeError::MartFilterUnsupportedKeys {
            mart,
            unsupported,
            promoted,
        }) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "mart_filter_unsupported_keys",
                "mart": mart,
                "unsupported_keys": unsupported,
                "promoted_keys": promoted,
                "hint": "Add the key to the mart's group_by, or query GET /api/entities/:id/history for sample-level reads."
            })),
        )
            .into_response(),
        Err(RuntimeError::MartNotFound { name }) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "mart_not_found", "mart": name})),
        )
            .into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

async fn create_sandbox(
    State(rt): State<Arc<WarehouseRuntime>>,
    Json(spec): Json<SandboxSpec>,
) -> impl IntoResponse {
    let cols = serde_json::to_value(&spec).unwrap();
    match rt.sandbox_define("user:rest", spec, cols).await {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

async fn drop_sandbox(
    State(rt): State<Arc<WarehouseRuntime>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match rt.sandbox_drop(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => runtime_error_response(e).into_response(),
    }
}

async fn run_gc(State(rt): State<Arc<WarehouseRuntime>>) -> impl IntoResponse {
    match crate::gc::run_once(&rt.pg, &rt.config).await {
        Ok(report) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "marts": report.marts,
                "cleaners": report.cleaners,
                "sandboxes": report.sandboxes,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn run_audit(State(rt): State<Arc<WarehouseRuntime>>) -> impl IntoResponse {
    match crate::audit::find_drift(&rt.pg).await {
        Ok(entries) => {
            (StatusCode::OK, Json(serde_json::json!({"drift": entries}))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

fn runtime_error_response(e: RuntimeError) -> (StatusCode, Json<serde_json::Value>) {
    let status = match &e {
        RuntimeError::MartNotFound { .. } => StatusCode::NOT_FOUND,
        RuntimeError::MartNotLive { .. } => StatusCode::CONFLICT,
        RuntimeError::SandboxFrozen { .. } => StatusCode::CONFLICT,
        RuntimeError::BadSpec(_) => StatusCode::BAD_REQUEST,
        RuntimeError::Catalog(crate::catalog::CatalogError::LiveMartQuotaExceeded { .. }) => {
            StatusCode::CONFLICT
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(serde_json::json!({"error": e.to_string()})))
}
