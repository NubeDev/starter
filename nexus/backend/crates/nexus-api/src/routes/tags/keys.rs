//! `GET /api/v1/tags/keys` — the distinct tag keys in use, for autocomplete.

use axum::extract::State;
use axum::response::IntoResponse as _;
use axum::{Extension, Json};
use nexus_store::tag;
use starter_server::error::IntoResponse;
use starter_spi::auth::Principal;

use crate::middleware::tenant::tenant_of;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/tags/keys",
    tag = "tags",
    operation_id = "list_tag_keys",
    responses((status = 200, description = "Distinct tag keys", body = [String])),
)]
pub async fn list_tag_keys(
    State(state): State<AppState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    let tenant = match tenant_of(&principal) {
        Ok(t) => t,
        Err(resp) => return resp,
    };
    match tag::distinct_keys(&state.metadata, &tenant).await {
        Ok(keys) => Json(keys).into_response(),
        Err(e) => IntoResponse(e).into_response(),
    }
}
