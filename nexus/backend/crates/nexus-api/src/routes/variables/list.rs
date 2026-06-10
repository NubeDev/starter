//! `GET /api/v1/dashboards/:slug/variables` — a dashboard's variables.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::variable::VariableDetail;
use nexus_store::{dashboard, variable};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::authz::{self, ACTION_VIEW, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/dashboards/{slug}/variables",
    tag = "variables",
    operation_id = "list_variables",
    params(("slug" = String, Path, description = "Dashboard slug (route alias)")),
    responses(
        (status = 200, description = "Variables in bar order", body = [VariableDetail]),
        (status = 404, description = "Dashboard not found in this tenant"),
    ),
)]
pub async fn list_variables(
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
        ACTION_VIEW,
        KIND_DASHBOARD,
        &dash.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match variable::list_for_dashboard(&state.metadata, &tenant, dash.id).await {
        Ok(vars) => Json(vars.iter().map(to_detail).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
