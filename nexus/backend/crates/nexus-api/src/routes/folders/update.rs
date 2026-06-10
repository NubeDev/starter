//! `PATCH /api/v1/folders/:id` — rename or reparent a folder.
//!
//! LAYER: transport (REST). Resolve → authorize → call domain → record → return.
//! Authorized as `edit` on the folder's immutable id.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::folder::{FolderSummary, UpdateFolderRequest};
use nexus_store::folder::{self, FolderPatch};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::changelog::Op;
use uuid::Uuid;

use super::convert::to_summary;
use super::recorded::{record_folder, snapshot};
use crate::authz::{self, ACTION_EDIT, KIND_FOLDER};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    patch,
    path = "/api/v1/folders/{id}",
    tag = "folders",
    operation_id = "update_folder",
    params(("id" = Uuid, Path, description = "Folder id")),
    request_body = UpdateFolderRequest,
    responses(
        (status = 200, description = "Folder updated", body = FolderSummary),
        (status = 400, description = "Invalid reparent (self-parent or absent parent)"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn update_folder(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFolderRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Read the current row first: it is both the authz subject (by immutable id)
    // and the `before` snapshot the undo log needs.
    let before = match folder::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(f)) => f,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_EDIT,
        KIND_FOLDER,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    // Collapse the JSON-friendly `parent_id` + `clear_parent` pair into the store's
    // three-valued patch: clear wins, then an explicit parent, else leave alone.
    let parent_id = if req.clear_parent {
        Some(None)
    } else {
        req.parent_id.map(Some)
    };
    let patch = FolderPatch {
        name: req.name,
        parent_id,
    };
    match folder::update(&state.metadata, &tenant, id, &patch).await {
        Ok(Some(rec)) => {
            record_folder(
                &state,
                principal,
                &tenant,
                Op::Update,
                &id.to_string(),
                Some(snapshot(&before)),
                Some(snapshot(&rec)),
            )
            .await;
            Json(to_summary(&rec)).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
