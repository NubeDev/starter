//! `POST /api/v1/insights` — create a stored insight for the caller's tenant.
//!
//! LAYER: transport (REST). Extract → validate → call domain → shape DTO → return.
//! No SQL, no business predicates here. See docs/design/layering/.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::insight::{CreateInsightRequest, InsightSummary};
use nexus_store::insight::{self, NewInsight};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use super::validate::compiles;
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/insights",
    tag = "insights",
    operation_id = "create_insight",
    request_body = CreateInsightRequest,
    responses(
        (status = 200, description = "Created", body = InsightSummary),
        (status = 400, description = "The script does not compile"),
    ),
)]
pub async fn create_insight(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateInsightRequest>,
) -> axum::response::Response {
    let (_principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Reject a script that will not compile *before* persisting, so a stored
    // insight is always at least syntactically runnable.
    if let Err(msg) = compiles(&req.script) {
        return (StatusCode::BAD_REQUEST, msg).into_response();
    }
    let new = NewInsight {
        name: req.name,
        script: req.script,
        params_schema: req.params_schema,
    };
    match insight::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_summary(&rec)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
