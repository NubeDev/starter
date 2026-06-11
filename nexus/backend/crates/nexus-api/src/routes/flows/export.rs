//! `GET /api/v1/flows/:id/export` — the portable flow JSON model.
//!
//! LAYER: transport (REST). Resolve → authorize (`view`) → redact secrets →
//! shape DTO. Emits a self-contained [`FlowExport`] (`name` + engine config)
//! that `POST /flows/import` can re-create from. Credentials embedded in the
//! input/output config are blanked before the model leaves the server so a
//! shared file never carries a password.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::flow::{redact_secrets, FlowExport, FLOW_SCHEMA_VERSION};
use nexus_store::flow;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use uuid::Uuid;

use crate::authz::{self, ACTION_VIEW, KIND_FLOW};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/flows/{id}/export",
    tag = "flows",
    operation_id = "export_flow",
    params(("id" = Uuid, Path, description = "Flow id")),
    responses(
        (status = 200, description = "Portable flow model", body = FlowExport),
        (status = 403, description = "Not allowed to view this flow"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn export_flow(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (caller, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let rec = match flow::get(&state.metadata, &tenant, id).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        caller,
        ACTION_VIEW,
        KIND_FLOW,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }

    // Redact credentials from a *copy* of the config (the stored flow is
    // untouched). `redacted` is the OR of both blobs so the UI can warn once.
    let mut input = rec.input.clone();
    let mut output = rec.output.clone();
    let redacted = redact_secrets(&mut input) | redact_secrets(&mut output);

    let export = FlowExport {
        schema_version: FLOW_SCHEMA_VERSION,
        name: rec.name,
        input,
        pipeline: rec.pipeline,
        output,
        redacted,
    };
    Json(export).into_response()
}
