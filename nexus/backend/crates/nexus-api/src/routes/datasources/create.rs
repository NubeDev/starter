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
    // SQL connectors fill the flat columns; stream/file kinds carry their
    // parameters in `config` and leave these defaulted (empty host, port 0).
    // The secret is only sealed when present, so a secret-less kind (parquet/csv)
    // stores no ciphertext.
    let new = NewDatasource {
        name: req.name,
        kind: kind_to_stored(req.kind).into(),
        host: req.host.unwrap_or_default(),
        port: req.port.map(i32::from).unwrap_or(0),
        database: req.database.unwrap_or_default(),
        db_user: req.user.unwrap_or_default(),
        secret: req.password.filter(|p| !p.is_empty()),
        config: req.config,
    };
    match datasource::insert(&state.metadata, &state.envelope, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
