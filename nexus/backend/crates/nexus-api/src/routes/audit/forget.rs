//! `POST /api/v1/audit/forget` — GDPR right-to-erasure for a user subject.
//!
//! LAYER: transport (REST).

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::audit::{ForgetRequest, ForgetResponse};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use nexus_store::changelog::NexusRecorder;

use super::gate::require_audit_read;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/audit/forget",
    tag = "audit",
    operation_id = "forget_subject",
    request_body = ForgetRequest,
    responses(
        (status = 200, description = "Rows tombstoned", body = ForgetResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not an admin / no tenant binding"),
    ),
)]
pub async fn forget_subject(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<ForgetRequest>,
) -> axum::response::Response {
    let tenant = match require_audit_read(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let recorder = NexusRecorder::new(state.metadata.clone(), tenant);
    match recorder.forget_actor(&req.subject).await {
        Ok(tombstoned) => Json(ForgetResponse { tombstoned }).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
