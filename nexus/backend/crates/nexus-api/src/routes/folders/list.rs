//! `GET /api/v1/folders` — list the caller's tenant's folders.
//!
//! LAYER: transport (REST). Extract → call domain → shape DTO → return.
//! No SQL, no business predicates, no cross-resource walks here.
//! See docs/design/layering/.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_spi::dto::folder::FolderSummary;
use nexus_store::folder;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use super::convert::to_summary;
use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/folders",
    tag = "folders",
    operation_id = "list_folders",
    responses((status = 200, description = "Folders in the tenant", body = [FolderSummary])),
)]
pub async fn list_folders(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match folder::list(&state.metadata, &tenant).await {
        Ok(rows) => Json(rows.iter().map(to_summary).collect::<Vec<_>>()).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
