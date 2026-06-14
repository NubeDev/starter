//! `GET /api/v1/insights` — list the caller's tenant's stored insights.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! See docs/design/layering/.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::insight::InsightSummary;
use nexus_store::insight;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/insights",
    tag = "insights",
    operation_id = "list_insights",
    responses((status = 200, description = "Insights in the tenant", body = [InsightSummary])),
)]
pub async fn list_insights(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match insight::list(&state.metadata, &tenant).await {
        Ok(rows) => Json(rows.iter().map(to_summary).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
