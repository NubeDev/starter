//! `PATCH /api/v1/dashboards/:slug` — rename or re-slug a dashboard.
//!
//! The patch is partial — omitted fields are left unchanged. Re-slugging changes
//! only the route alias; grants and panel refs key on the immutable id, so
//! nothing is orphaned. Authorized as `edit` on the dashboard, like a panel edit.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{DashboardSummary, UpdateDashboardRequest};
use nexus_store::dashboard::{self, DashboardPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/dashboards/{slug}",
    tag = "dashboards",
    operation_id = "update_dashboard",
    params(("slug" = String, Path, description = "Dashboard slug")),
    request_body = UpdateDashboardRequest,
    responses(
        (status = 200, description = "Dashboard updated", body = DashboardSummary),
        (status = 404, description = "Not found in this tenant"),
        (status = 409, description = "New slug already used in this tenant"),
    ),
)]
pub async fn update_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
    Json(req): Json<UpdateDashboardRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Resolve the slug to the immutable id (a 404 if RLS hides it), then
    // authorize `edit` on that dashboard before mutating it.
    let dash = match dashboard::by_slug(&state.metadata, &tenant, &slug).await {
        Ok(Some(d)) => d,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let patch = DashboardPatch {
        name: req.name,
        slug: req.slug,
    };
    match dashboard::update(&state.metadata, &tenant, dash.id, &patch).await {
        Ok(Some(rec)) => Json(to_summary(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
