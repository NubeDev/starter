//! `DELETE /api/v1/panels/:id` — remove a panel.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::dashboard;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/panels/{id}",
    tag = "panels",
    operation_id = "delete_panel",
    params(("id" = Uuid, Path, description = "Panel id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_panel(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match dashboard::panel::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
