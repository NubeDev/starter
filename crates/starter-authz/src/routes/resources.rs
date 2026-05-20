//! `GET /v1/authz/resources` — enumerate every registered
//! resource kind. The admin UI uses the response to render the
//! permissions grid (SCOPE.md "Permissions grid").

use std::sync::Arc;

use axum::extract::Extension;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::state::AuthzRoutesState;

pub(super) async fn list_resources(Extension(state): Extension<Arc<AuthzRoutesState>>) -> Response {
    let known = state.registry.known();
    Json(serde_json::json!({ "resources": known })).into_response()
}
