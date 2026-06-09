//! `POST /api/v1/dashboards/:slug/variables` — define a variable on a dashboard.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::variable::{CreateVariableRequest, VariableDetail};
use nexus_store::variable::{self, NewVariable};
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::{kind_str, to_detail};
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/dashboards/{slug}/variables",
    tag = "variables",
    operation_id = "create_variable",
    params(("slug" = String, Path, description = "Dashboard slug (route alias)")),
    request_body = CreateVariableRequest,
    responses(
        (status = 200, description = "Created", body = VariableDetail),
        (status = 404, description = "Dashboard not found in this tenant"),
        (status = 409, description = "Variable name already used on this dashboard"),
    ),
)]
pub async fn create_variable(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(slug): Path<String>,
    Json(req): Json<CreateVariableRequest>,
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
    // Defining a variable mutates the dashboard — require `edit` on it.
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
    let new = NewVariable {
        dashboard_id: dash.id,
        name: req.name,
        label: req.label,
        kind: kind_str(req.kind).to_string(),
        options_config: req.options_config,
        current: req.current,
        multi: req.multi,
        include_all: req.include_all,
        hidden: req.hidden,
        sort_order: req.sort_order,
    };
    match variable::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
