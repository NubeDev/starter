//! `POST /api/v1/folders` — create a folder for the caller's tenant.
//!
//! LAYER: transport (REST). Extract → call domain → record → shape DTO → return.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::folder::{CreateFolderRequest, FolderSummary};
use nexus_store::folder::{self, NewFolder};
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;
use starter_spi::changelog::Op;

use super::convert::to_summary;
use super::recorded::{record_folder, snapshot};
use crate::middleware::tenant::caller;
use crate::state::AppState;

#[utoipa::path(
    post,
    path = "/api/v1/folders",
    tag = "folders",
    operation_id = "create_folder",
    request_body = CreateFolderRequest,
    responses(
        (status = 200, description = "Created", body = FolderSummary),
        (status = 400, description = "Parent folder does not exist in this tenant"),
    ),
)]
pub async fn create_folder(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
    Json(req): Json<CreateFolderRequest>,
) -> axum::response::Response {
    let (principal, tenant) = match caller(&principal) {
        Ok(c) => c,
        Err(resp) => return resp,
    };
    let new = NewFolder {
        parent_id: req.parent_id,
        name: req.name,
    };
    match folder::insert(&state.metadata, &tenant, &new).await {
        Ok(rec) => {
            record_folder(
                &state,
                principal,
                &tenant,
                Op::Create,
                &rec.id.to_string(),
                None,
                Some(snapshot(&rec)),
            )
            .await;
            Json(to_summary(&rec)).into_response()
        }
        Err(e) => IntoResponse(e).into_response(),
    }
}
