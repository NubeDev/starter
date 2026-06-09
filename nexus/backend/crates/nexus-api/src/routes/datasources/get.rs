//! `GET /api/v1/datasources/:id` — one datasource, redacted.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::DatasourceDetail;
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use super::convert::to_detail;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/datasources/{id}",
    tag = "datasources",
    operation_id = "get_datasource",
    params(("id" = Uuid, Path, description = "Datasource id")),
    responses(
        (status = 200, description = "Datasource", body = DatasourceDetail),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn get_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(rec)) => Json(to_detail(&rec)).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
