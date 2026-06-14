//! `GET /api/v1/nexus-db/schema` — admin read-only introspection of the
//! control-plane (metadata) DB's tables, columns, and foreign keys.
//!
//! The sibling of [`super::query`]: same admin-only + tenant-bound + read-only
//! gates, but instead of running caller SQL it runs the server-owned
//! introspection over `information_schema`. It returns the same
//! [`DatasourceSchema`] wire shape the datasource schema route returns, so the
//! frontend's ER diagram renders the metadata DB exactly like any datasource.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::datasource::DatasourceSchema;
use starter_server::error::IntoResponse;
use starter_spi::auth::{Principal, Role};

use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/nexus-db/schema",
    tag = "nexus-db",
    operation_id = "nexus_db_schema",
    responses(
        (status = 200, description = "Tables, columns, and foreign keys", body = DatasourceSchema),
        (status = 403, description = "Caller is not an admin"),
    ),
)]
pub async fn nexus_db_schema(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    // Tenant binding first (401 unauthenticated / 403 no tenant), then admin
    // gate — identical to the query handler so the two stay in lockstep.
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    if principal.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "admin only").into_response();
    }
    match nexus_store::introspect_tenant_ro(&state.metadata, &tenant, state.guards).await {
        Ok(info) => Json(crate::routes::schema_dto::to_dto(info)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
