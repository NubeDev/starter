//! `DELETE /api/v1/insights/:id` — remove a stored insight.
//!
//! LAYER: transport (REST). Resolve → authorize → call domain → return.
//! Authorized as `delete` on the insight's immutable id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::insight;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_INSIGHT};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/insights/{id}",
    tag = "insights",
    operation_id = "delete_insight",
    params(("id" = Uuid, Path, description = "Insight id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 403, description = "Not authorized to delete this insight"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_insight(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_DELETE,
        KIND_INSIGHT,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match insight::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
