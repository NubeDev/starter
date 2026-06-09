//! `POST /api/v1/datasources/:id/query` — run one SQL statement against a
//! registered datasource.
//!
//! Unlike the dev `POST /query`, this resolves the *caller's own* datasource:
//! it checks the tenant can `view` it (the same grant gate as the read routes,
//! D6), builds (or reuses) a pool to that datasource through the audited decrypt
//! boundary, and runs the SQL under the R4 guards. The handler only wires — the
//! connection and the guards live in the store and the pool cache.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use nexus_store::datasource;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_DATASOURCE};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/datasources/{id}/query",
    tag = "datasources",
    operation_id = "query_datasource",
    params(("id" = Uuid, Path, description = "Datasource id")),
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Invalid or rejected query"),
        (status = 403, description = "Not authorized to view this datasource"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn query_datasource(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<QueryRequest>,
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
    let identity = nexus_store::QueryIdentity {
        tenant_id: Some(tenant.clone()),
        user_id: Some(caller_principal.subject.clone()),
    };
    let result = nexus_store::run_request(&pool, &req, &identity, state.guards).await;
    record(&state, &tenant, &caller_principal.subject, id, &req.sql, &result).await;
    match result {
        Ok(out) => Json(out).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}

/// Persist the run to query history (best-effort: a history write failure must
/// not fail the query, so its error is logged, not propagated). The stats →
/// record mapping is a plain DTO shaping, not business logic.
async fn record(
    state: &AppState,
    tenant: &str,
    user_id: &str,
    datasource_id: Uuid,
    sql: &str,
    result: &Result<nexus_spi::dto::query::QueryResponse, starter_spi::Error>,
) {
    let run = match result {
        Ok(out) => nexus_store::query_history::NewQueryRun {
            user_id: user_id.to_string(),
            datasource_id: Some(datasource_id),
            sql: sql.to_string(),
            elapsed_ms: Some(out.stats.elapsed_ms as i64),
            row_count: Some(out.stats.row_count as i64),
            error: None,
        },
        Err(e) => nexus_store::query_history::NewQueryRun {
            user_id: user_id.to_string(),
            datasource_id: Some(datasource_id),
            sql: sql.to_string(),
            elapsed_ms: None,
            row_count: None,
            error: Some(e.to_string()),
        },
    };
    if let Err(e) = nexus_store::query_history::record_run(&state.metadata, tenant, &run).await {
        tracing::warn!(error = %e, "failed to record query history");
    }
}
