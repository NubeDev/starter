//! `DELETE /api/v1/dashboards/:slug` — remove a dashboard and its panels.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use crate::authz::{self, ACTION_DELETE, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/dashboards/{slug}",
    tag = "dashboards",
    operation_id = "delete_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match dashboard::delete(&state.metadata, &tenant, dash.id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
