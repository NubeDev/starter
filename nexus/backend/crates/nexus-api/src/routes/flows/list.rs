//! `GET /api/v1/flows` — the tenant's flows.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::FlowSummary;
use nexus_store::flow;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/flows",
    tag = "flows",
    operation_id = "list_flows",
    responses((status = 200, description = "Flows", body = [FlowSummary])),
)]
pub async fn list_flows(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match flow::list(&state.metadata, &tenant).await {
        Ok(recs) => {
            let out: Vec<FlowSummary> = recs.iter().map(|r| to_summary(r, &state.flows)).collect();
            Json(out).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
