//! `DELETE /api/v1/folders/:id` — remove a folder, re-rooting its contents.
//!
//! LAYER: transport (REST). Resolve → authorize → call domain → record → return.
//! Child folders and filed dashboards are re-rooted by the store, never deleted.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse as _;
use axum::Extension;
use nexus_store::folder;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::changelog::Op;
use uuid::Uuid;

use super::recorded::{record_folder, snapshot};
use crate::authz::{self, ACTION_DELETE, KIND_FOLDER};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    delete,
    path = "/api/v1/folders/{id}",
    tag = "folders",
    operation_id = "delete_folder",
    params(("id" = Uuid, Path, description = "Folder id")),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found in this tenant"),
    ),
)]
pub async fn delete_folder(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Path(id): Path<Uuid>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    // Read the row before deleting: the authz subject and the `before` snapshot.
    let before = match folder::by_id(&state.metadata, &tenant, id).await {
        Ok(Some(f)) => f,
        Ok(None) => return (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => return IntoResponse(e).into_response(),
    };
    if let Err(resp) = authz::require(
        state.engine.as_ref(),
        principal,
        ACTION_DELETE,
        KIND_FOLDER,
        &id.to_string(),
        &tenant,
    )
    .await
    {
        return resp;
    }
    match folder::delete(&state.metadata, &tenant, id).await {
        Ok(true) => {
            record_folder(
                &state,
                principal,
                &tenant,
                Op::Delete,
                &id.to_string(),
                Some(snapshot(&before)),
                None,
            )
            .await;
            StatusCode::NO_CONTENT.into_response()
        }
        Ok(false) => (StatusCode::NOT_FOUND, "not found").into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
