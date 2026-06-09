//! `GET /api/v1/dashboards/:slug` — one dashboard with its panels.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::DashboardDetail;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{slug}",
    tag = "dashboards",
    operation_id = "get_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug (route alias)")),
    responses(
        (status = 200, description = "Dashboard with panels", body = DashboardDetail),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve the slug to the immutable id, then load by id.
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    // The grant check keys on the immutable id (resolved above), never the slug.
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match dashboard::panel::list_for_dashboard(&state.metadata, &tenant, dash.id).await {
        Ok(panels) => Json(to_detail(&dash, &panels)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
