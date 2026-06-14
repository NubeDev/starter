//! `POST /api/v1/flows/import` — re-create a flow from a previously exported
//! model.
//!
//! LAYER: transport (REST). Validate `schema_version` → mint a fresh flow in the
//! caller's tenant. The imported flow always lands stopped (`enabled = false`):
//! a shared file should never auto-start on import, and any redacted credentials
//! must be re-entered (and the flow started) deliberately. Returns the new
//! [`FlowDetail`] so the UI can route straight into the editor.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::{FlowDetail, FlowExport, FLOW_SCHEMA_VERSION};
use nexus_store::flow::{self, NewFlow};
use serde_json::json;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_detail;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/flows/import",
    tag = "flows",
    operation_id = "import_flow",
    request_body = FlowExport,
    responses(
        (status = 200, description = "Imported", body = FlowDetail),
        (status = 400, description = "Unknown schema_version"),
    ),
)]
pub async fn import_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(model): Json<FlowExport>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    if model.schema_version != FLOW_SCHEMA_VERSION {
        return (
            StatusCode::BAD_REQUEST,
            format!(
                "unsupported flow schema_version {} (this server understands {})",
                model.schema_version, FLOW_SCHEMA_VERSION
            ),
        )
            .into_response();
    }

    // A pipeline omitted from the file (or sent as null) becomes an empty
    // processor list, matching `create_flow`'s default.
    let pipeline = if model.pipeline.is_null() {
        json!([])
    } else {
        model.pipeline
    };

    let new = NewFlow {
        name: model.name,
        input: model.input,
        pipeline,
        output: model.output,
        // Always import stopped — never auto-start someone else's flow, and
        // redacted credentials must be re-entered first.
        enabled: false,
    };
    match flow::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => Json(to_detail(&rec, &state.flows)).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
