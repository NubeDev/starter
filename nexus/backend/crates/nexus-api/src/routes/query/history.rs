//! `GET /api/v1/query-history` and `POST /api/v1/query-history/:id/star` — the
//! caller's recent query runs, and pinning one as a favourite.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query_history::{QueryHistoryEntry, QueryHistoryList, StarQueryRequest};
use nexus_store::query_history;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::middleware::tenant::caller;
use crate::state::AppState;

/// How many recent runs the drawer loads. A fixed bound keeps the response small
/// and is not caller-tunable (matching the query-guard philosophy).
const HISTORY_LIMIT: i64 = 100;

#[utoipa::path(
    get,
    path = "/api/v1/query-history",
    tag = "query",
    operation_id = "list_query_history",
    responses(
        (status = 200, description = "Recent query runs", body = QueryHistoryList),
        (status = 401, description = "Unauthenticated"),
    ),
)]
pub async fn list_query_history(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (p, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match query_history::list_recent(&state.metadata, &tenant, &p.subject, HISTORY_LIMIT).await {
        Ok(rows) => Json(QueryHistoryList {
            entries: rows.into_iter().map(to_entry).collect(),
        })
        .into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/query-history/{id}/star",
    tag = "query",
    operation_id = "star_query_history",
    params(("id" = Uuid, Path, description = "History row id")),
    request_body = StarQueryRequest,
    responses(
        (status = 204, description = "Star state updated"),
        (status = 404, description = "Not this user's history row"),
    ),
)]
pub async fn star_query_history(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<StarQueryRequest>,
) -> axum::response::Response {
    let (p, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    match query_history::set_starred(&state.metadata, &tenant, &p.subject, id, req.starred).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Map a store history row to the wire entry.
fn to_entry(r: nexus_store::query_history::QueryHistoryRow) -> QueryHistoryEntry {
    QueryHistoryEntry {
        id: r.id,
        datasource_id: r.datasource_id,
        sql: r.sql,
        ran_at: r.ran_at,
        elapsed_ms: r.elapsed_ms,
        row_count: r.row_count,
        error: r.error,
        starred: r.starred,
    }
}
