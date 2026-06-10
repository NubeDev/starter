//! `GET /api/v1/query-kinds/:id` — one tenant-authored query-kind in full.

use axum::extract::{Path, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query_kind::QueryKindDetail;
use nexus_store::query_kind;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_QUERY_KIND};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/query-kinds/{id}",
    tag = "query-kinds",
    operation_id = "get_query_kind",
    params(("id" = Uuid, Path, description = "Query-kind id")),
    responses(
        (status = 200, description = "Query-kind", body = QueryKindDetail),
        (status = 403, description = "Not allowed to view this query-kind"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_query_kind(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // The store maps an absent (or RLS-hidden) id to `NotFound`, which the error
    // shim renders as a 404 — no separate `Option` branch as for agents.
    let rec = match query_kind::get(&state.metadata, &tenant, id).await {
        Ok(r) => r,
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_QUERY_KIND,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_detail(&rec)).into_response()
}
