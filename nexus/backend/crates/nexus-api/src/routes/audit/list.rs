//! `GET /api/v1/audit` — filtered, paged, newest-first audit list.
//!
//! LAYER: transport (REST).

use axum::extract::{Query, State};
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use starter_changelog::{ChangeFilter, ChangeLog as _, ChangePage};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use nexus_store::changelog::NexusChangeLog;

use super::gate::require_audit_read;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/audit",
    tag = "audit",
    operation_id = "list_audit",
    params(ChangeFilter),
    responses(
        (status = 200, description = "Audit page (newest first)", body = ChangePage),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "Not an admin / no tenant binding"),
    ),
)]
pub async fn list_audit(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Query(filter): Query<ChangeFilter>,
) -> axum::response::Response {
    let tenant = match require_audit_read(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    let log = NexusChangeLog::new(state.metadata.clone(), tenant);
    match log.list(&filter).await {
        Ok(page) => Json(page).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
