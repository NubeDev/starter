//! `DELETE /api/v1/flows/:id` — remove a flow, stopping it first if running.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::flow;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/flows/{id}",
    tag = "flows",
    operation_id = "delete_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not allowed to delete this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_FLOW,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match flow::delete(&state.metadata, &tenant, id).await {
        // Stop the running stream on the way out so deleting a flow also halts it.
        Ok(true) => {
            state.flows.stop(&id.to_string());
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
