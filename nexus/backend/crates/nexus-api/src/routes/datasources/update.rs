//! `PUT /api/v1/datasources/:id` — update a datasource in the caller's tenant.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::{DatasourceDetail, UpdateDatasourceRequest};
use nexus_store::datasource::{self, DatasourcePatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::authz::{self, ACTION_EDIT, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    put,
    path = "/api/v1/datasources/{id}",
    tag = "datasources",
    operation_id = "update_datasource",
    params(("id" = Uuid, Path, description = "Datasource id")),
    request_body = UpdateDatasourceRequest,
    responses(
        (status = 200, description = "Updated", body = DatasourceDetail),
        (status = 403, description = "Not authorized to edit this datasource"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateDatasourceRequest>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Existence (RLS) before authorization, so a missing row is a 404 and a
    // forbidden one a 403 — keyed on the immutable id, like get/delete.
    match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(_)) => {}
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    }
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_EDIT,
        KIND_DATASOURCE,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    let patch = DatasourcePatch {
        name: req.name,
        host: req.host,
        port: req.port.map(|p| p as i32),
        database: req.database,
        db_user: req.user,
        secret: req.password,
    };
    match datasource::update(&state.metadata, &state.envelope, &tenant, id, &patch).await {
        Ok(true) => {}
        Ok(false) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    }

    // Any cached pool may now point at stale host/credentials — drop it so the
    // next query rebuilds against the updated connection.
    state.datasource_pools.evict(&tenant, id).await;

    match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(rec)) => Json(to_detail(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
