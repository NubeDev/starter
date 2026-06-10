//! `POST /api/v1/query` — run one SQL statement against the datasource.

use axum::extract::State;
use axum::Json;
use nexus_spi::dto::query::{QueryRequest, QueryResponse};
use starter_server::error::IntoResponse;

use crate::state::AppState;

/// Extract the request, bind its macros/variables, run it under the server
/// guards, return the rows. The dev single-datasource shortcut carries no
/// principal, so host tokens (`$caller_tenant_id`) are absent — a query needing
/// them errors, which is correct. The guards (read-only, timeout, caps) and the
/// binder live in the store; this handler only wires.
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
    let identity = nexus_store::QueryIdentity::default();
    let result = crate::cache::run_cached(&state, &state.datasource, &req, &identity, "dev")
        .await
        .map_err(IntoResponse)?;
    // RW-06 insight seam (dev path): no tenant context, so only an inline script
    // is valid; a stored reference is a clean caller error, not a panic.
    let result = match &req.insight {
        Some(insight) => {
            crate::insights::apply_insight(&state, &state.metadata, None, insight, result)
                .await
                .map_err(IntoResponse)?
        }
        None => result,
    };
    Ok(Json(result))
}
