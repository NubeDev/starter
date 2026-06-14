//! `GET /api/v1/nav/{id}` — one nav node, gated on `view` of the node itself.
//!
//! Opening a node is a node `view` check (WS-13 §6) — *not* a check on the page
//! it mounts. That is the whole access model: the node is what a user navigates,
//! so the node is what's granted. The page's own grants gate authoring, not
//! navigation.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::nav::NavNodeDetail;
use nexus_store::nav_node;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_NAV_NODE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/nav/{id}",
    tag = "nav",
    operation_id = "get_nav",
    params(("id" = Uuid, Path, description = "Nav node id")),
    responses(
        (status = 200, description = "The nav node", body = NavNodeDetail),
        (status = 403, description = "Not allowed to view this node"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_nav(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve under RLS first: a 404 for absent/foreign so existence isn't
    // leaked, then a node `view` check before returning it.
    let node = match nav_node::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(n)) => n,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_NAV_NODE,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    Json(to_detail(&node)).into_response()
}
