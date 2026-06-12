//! `POST /api/v1/nexus-db/query` — admin read-only SQL against the metadata DB.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query::QueryResponse;
use serde::Deserialize;
use starter_server::error::IntoResponse;
use starter_spi::auth::{Principal, Role};
use utoipa::ToSchema;

use crate::middleware::tenant::caller;
use crate::state::AppState;

/// The request body: just the SQL to run. No datasource id — the target is the
/// control-plane metadata pool, fixed server-side. No params/federation/insight;
/// this is a plain read-only inspector.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct NexusDbQueryRequest {
    /// Raw read-only SQL. Writes and DDL are rejected by the read-only
    /// transaction, and rows are RLS-filtered to the caller's tenant.
    pub sql: String,
}

#[utoipa::path(
    post,
    path = "/api/v1/nexus-db/query",
    tag = "nexus-db",
    operation_id = "query_nexus_db",
    request_body = NexusDbQueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Malformed SQL or a rejected write"),
        (status = 403, description = "Caller is not an admin"),
    ),
)]
pub async fn query_nexus_db(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<NexusDbQueryRequest>,
) -> axum::response::Response {
    // Tenant binding first (401 unauthenticated / 403 no tenant), mirroring every
    // other tenant-scoped handler.
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // The metadata DB is platform internals — admin only. A reader/writer is
    // refused before any SQL touches the pool.
    if principal.role != Role::Admin {
        return (StatusCode::FORBIDDEN, "admin only").into_response();
    }
    match nexus_store::query::run_query_tenant_ro(&state.metadata, &tenant, &req.sql, state.guards)
        .await
    {
        Ok(result) => Json(result).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
