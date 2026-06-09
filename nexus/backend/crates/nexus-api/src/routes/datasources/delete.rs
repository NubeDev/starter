//! `DELETE /api/v1/datasources/:id` — remove a datasource in the caller's tenant.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/datasources/{id}",
    tag = "datasources",
    operation_id = "delete_datasource",
    params(("id" = Uuid, Path, description = "Datasource id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match datasource::delete(&state.metadata, &tenant, id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
