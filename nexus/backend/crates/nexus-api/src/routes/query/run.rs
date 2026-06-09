//! `POST /api/v1/query` — run one SQL statement against the datasource.

use axum::extract::State;
use axum::Json;
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use starter_server::error::IntoResponse;

use crate::state::AppState;

/// Extract the SQL, run it under the server guards, return the rows. The guards
/// (read-only, timeout, caps) live in the store; this handler only wires.
#[utoipa::path(
    post,
    path = "/api/v1/query",
    tag = "query",
    operation_id = "run_query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query result", body = QueryResponse),
        (status = 400, description = "Invalid or rejected query", body = nexus_spi::Problem),
    ),
)]
pub async fn run_query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, IntoResponse> {
    let result = nexus_store::run_query(&state.datasource, &req.sql, state.guards)
        .await
        .map_err(IntoResponse)?;
    Ok(Json(result))
}
