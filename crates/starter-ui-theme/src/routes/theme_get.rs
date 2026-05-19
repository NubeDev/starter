//! `GET /api/v1/ui/theme` — return the stored [`ThemeDocument`].

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_spi::ui::theme::ThemeDocument;

use super::errors::map_internal;
use super::guards::require_authenticated;
use super::state::ThemeState;

#[utoipa::path(
    get,
    path = "/api/v1/ui/theme",
    tag = "ui-theme",
    operation_id = "ui_theme_get",
    responses(
        (status = 200, description = "Current theme document", body = ThemeDocument),
        (status = 401, description = "Authentication required"),
    ),
)]
pub(crate) async fn get_theme(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    if let Some(resp) = require_authenticated(&req) {
        return resp;
    }
    match state.store.load().await {
        Ok(doc) => (axum::http::StatusCode::OK, Json(doc)).into_response(),
        Err(e) => map_internal(e),
    }
}
