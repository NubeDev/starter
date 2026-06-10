//! `PATCH /api/v1/variables/:id` — edit a variable's definition or selection.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::variable::{UpdateVariableRequest, VariableDetail};
use nexus_store::variable::{self, VariablePatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::{kind_str, to_detail};
use crate::authz::{self, ACTION_EDIT, KIND_DASHBOARD};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/variables/{id}",
    tag = "variables",
    operation_id = "update_variable",
    params(("id" = Uuid, Path, description = "Variable id")),
    request_body = UpdateVariableRequest,
    responses(
        (status = 200, description = "Updated", body = VariableDetail),
        (status = 404, description = "Not found in this tenant"),
        (status = 409, description = "Variable name already used on this dashboard"),
    ),
)]
pub async fn update_variable(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateVariableRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Editing a variable mutates its dashboard — authorize `edit` on the owning
    // dashboard, resolved from the variable. A variable RLS-hidden from the
    // tenant is a 404.
    let owning = match variable::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(v)) => v.dashboard_id,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DASHBOARD,
        &owning.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let patch = VariablePatch {
        name: req.name,
        label: req.label,
        kind: req.kind.map(|k| kind_str(k).to_string()),
        options_config: req.options_config,
        current: req.current,
        multi: req.multi,
        include_all: req.include_all,
        hidden: req.hidden,
        sort_order: req.sort_order,
    };
    match variable::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => Json(to_detail(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
