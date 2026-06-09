//! `GET /api/v1/dashboards` — list the caller's tenant's dashboards.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::DashboardSummary;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    operation_id = "list_dashboards",
    responses((status = 200, description = "Dashboards", body = [DashboardSummary])),
)]
pub async fn list_dashboards(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match dashboard::list(&state.metadata, &tenant).await {
        Ok(rows) => Json(rows.iter().map(to_summary).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
