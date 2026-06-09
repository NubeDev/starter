//! `POST /api/v1/dashboards` — create a dashboard for the caller's tenant.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::{CreateDashboardRequest, DashboardSummary};
use nexus_store::dashboard::{self, NewDashboard};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    operation_id = "create_dashboard",
    request_body = CreateDashboardRequest,
    responses(
        (status = 200, description = "Created", body = DashboardSummary),
        (status = 409, description = "Slug already used in this tenant"),
    ),
)]
pub async fn create_dashboard(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateDashboardRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewDashboard {
        slug: req.slug,
        name: req.name,
    };
    match dashboard::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_summary(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
