//! `POST /api/v1/undo` and `POST /api/v1/redo` handlers.
//!
//! LAYER: transport (REST).

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::audit::UndoResponse;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::changelog::GroupId;

use crate::changelog::{actor_from, undo_service_for};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/undo",
    tag = "audit",
    operation_id = "undo",
    responses(
        (status = 200, description = "Group that was undone", body = UndoResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "No tenant binding"),
        (status = 404, description = "No undoable group for this actor"),
        (status = 409, description = "Stale resource version — refused"),
    ),
)]
pub async fn undo(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    apply(&state, &principal, Direction::Undo).await
}

#[utoipa::path(
    post,
    path = "/api/v1/redo",
    tag = "audit",
    operation_id = "redo",
    responses(
        (status = 200, description = "Group that was redone", body = UndoResponse),
        (status = 401, description = "Unauthenticated"),
        (status = 403, description = "No tenant binding"),
        (status = 404, description = "Redo stack empty"),
        (status = 409, description = "Stale resource version — refused"),
    ),
)]
pub async fn redo(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    apply(&state, &principal, Direction::Redo).await
}

/// Which way the cursor walks. Both directions share the per-request service
/// build and the actor/tenant resolution, differing only in the service call.
enum Direction {
    Undo,
    Redo,
}

/// Resolve the caller, build the tenant-pinned [`starter_undo::UndoService`] from
/// the boot-shared handles, and apply one step. A missing tenant binding is a 403
/// (fail-closed), matching every other tenant-scoped route.
async fn apply(
    state: &AppState,
    principal: &Option<Extension<Principal>>,
    direction: Direction,
) -> axum::response::Response {
    let (principal, tenant) = match caller(principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let actor = actor_from(principal);
    let service = undo_service_for(&state.changelog, state.metadata.clone(), &tenant);
    let result: starter_spi::Result<GroupId> = match direction {
        Direction::Undo => service.undo(&actor).await,
        Direction::Redo => service.redo(&actor).await,
    };
    match result {
        Ok(group) => Json(UndoResponse { group_id: group.0 }).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
