//! `GET /api/v1/datasources/:id/schema` — introspect a datasource's tables and
//! columns for editor autocomplete.
//!
//! Same gates as the query route (D6): the caller must be able to `view` the
//! datasource, the connection is built through the audited decrypt boundary, and
//! the introspection runs under the R4 read-only guards. It returns catalog
//! metadata only — table and column names — never row data.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::DatasourceSchema;
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/datasources/{id}/schema",
    tag = "datasources",
    operation_id = "datasource_schema",
    params(("id" = Uuid, Path, description = "Datasource id")),
    responses(
        (status = 200, description = "Tables and columns", body = DatasourceSchema),
        (status = 403, description = "Not authorized to view this datasource"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn datasource_schema(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller_principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match datasource::get(&state.metadata, &tenant, id).await {
        Ok(Some(rec)) => rec,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller_principal,
        ACTION_VIEW,
        KIND_DATASOURCE,
        &rec.id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    let pool = match state
        .datasource_pools
        .get_or_connect(
            &state.metadata,
            &state.envelope,
            &tenant,
            &caller_principal.subject,
            &rec,
        )
        .await
    {
        Ok(p) => p,
        Err(e) => return IntoResponse(e).into_response(),
    };
    match nexus_store::introspect(&pool, state.guards).await {
        Ok(info) => Json(crate::routes::schema_dto::to_dto(info)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
