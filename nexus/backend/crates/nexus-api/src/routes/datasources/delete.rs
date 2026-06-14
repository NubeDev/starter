//! `DELETE /api/v1/datasources/:id` — remove a datasource in the caller's tenant.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_DELETE, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
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
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Confirm the datasource is visible (RLS) before authorizing, so a missing
    // row is a 404 and a forbidden one a 403 — keyed on the immutable id.
    match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_DELETE,
        KIND_DATASOURCE,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match datasource::delete(&state.metadata, &tenant, id).await {
        Ok(true) => {
            // Drop any cached pool so a recreated id never reuses a stale one.
            state.datasource_pools.evict(&tenant, id).await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
