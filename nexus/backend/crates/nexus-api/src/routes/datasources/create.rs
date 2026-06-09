//! `POST /api/v1/datasources` — register a datasource for the caller's tenant.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::{CreateDatasourceRequest, DatasourceDetail};
use nexus_store::datasource::{self, NewDatasource};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::{kind_to_stored, to_detail};
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

/// Seal the secret and insert the datasource under the caller's tenant.
#[utoipa::path(
    post,
    path = "/api/v1/datasources",
    tag = "datasources",
    operation_id = "create_datasource",
    request_body = CreateDatasourceRequest,
    responses((status = 200, description = "Created", body = DatasourceDetail)),
)]
pub async fn create_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateDatasourceRequest>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let new = NewDatasource {
        name: req.name,
        kind: kind_to_stored(req.kind).into(),
        host: req.host,
        port: req.port as i32,
        database: req.database,
        db_user: req.user,
        secret: req.password,
    };
    match datasource::insert(&state.metadata, &state.envelope, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
