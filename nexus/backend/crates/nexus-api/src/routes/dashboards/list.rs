//! `GET /api/v1/dashboards` — list the caller's tenant's dashboards.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::dashboard::DashboardSummary;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::authz::{self, ACTION_VIEW, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/dashboards",
    tag = "dashboards",
    operation_id = "list_dashboards",
    responses((status = 200, description = "Dashboards the caller may view", body = [DashboardSummary])),
)]
pub async fn list_dashboards(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // RLS already scopes the rows to the tenant; here we further drop the ones
    // the caller can't `view`, so a dashboard shared only with specific people
    // never appears in another member's list (and so the sidebar that renders
    // this list hides it). This is the bulk twin of the per-id `view` gate that
    // `get_dashboard` enforces — same engine check, so the list and the open
    // path agree on visibility.
    let rows = match dashboard::list(&state.metadata, &tenant).await {
        Ok(rows) => rows,
        Err(e) => return IntoResponse(e).into_response(),
    };
    let mut visible = Vec::with_capacity(rows.len());
    for row in &rows {
        if authz::can(
            state.engine.as_ref(),
            caller,
            ACTION_VIEW,
            KIND_DASHBOARD,
            &row.id.to_string(),
            &tenant,
        )
        .await
        {
            visible.push(to_summary(row));
        }
    }
    Json(visible).into_response()
}
