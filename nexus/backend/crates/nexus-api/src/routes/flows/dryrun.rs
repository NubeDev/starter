//! `POST /api/v1/flows/dry-run` — validate + bounded test run, no persistence.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::http::StatusCode;
use axum::{Extension, Json};
use nexus_spi::dto::flow::{DryRunRequest, DryRunResponse};
use serde_json::json;
use starter_spi::auth::Principal;

use crate::flows::dry_run;
use crate::middleware::tenant::tenant_of;

/// Run the supplied input + pipeline against the bounded collector and return
/// the sample (or an inline build/runtime error). Principal-gated: a dry run
/// executes a flow against real connectors, so it sits behind the same auth
/// boundary as a saved flow. The pipeline defaults to empty when omitted.
#[utoipa::path(
    post,
    path = "/api/v1/flows/dry-run",
    tag = "flows",
    operation_id = "dry_run_flow",
    request_body = DryRunRequest,
    responses(
        (status = 200, description = "Dry-run sample or inline error", body = DryRunResponse),
        (status = 500, description = "Engine failed to initialise"),
    ),
)]
pub async fn dry_run_flow(
    principal: Option<Extension<Principal>>,
    Json(req): Json<DryRunRequest>,
) -> Result<Json<DryRunResponse>, (StatusCode, String)> {
    if let Err(_resp) = tenant_of(&principal) {
        return Err((StatusCode::UNAUTHORIZED, "authentication required".into()));
    }
    let processors = req
        .pipeline
        .and_then(|p| p.as_array().cloned())
        .unwrap_or_else(|| json!([]).as_array().cloned().unwrap_or_default());
    dry_run::run(req.input, processors, req.max_rows)
        .await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))
}
