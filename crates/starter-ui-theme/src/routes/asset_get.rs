//! `GET /api/v1/ui/theme/{logo,favicon}` — serve the stored bytes
//! with the correct `Content-Type`. Public on purpose: browsers
//! request these via `<img src>` / `<link rel="icon">`, which can't
//! carry an `Authorization` header, and the assets are non-sensitive
//! by nature (the org's public logo).

use std::sync::Arc;

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response as HttpResponse, StatusCode};
use axum::response::{IntoResponse, Response};

use super::errors::map_internal;
use super::state::ThemeState;

#[utoipa::path(
    get,
    path = "/api/v1/ui/theme/logo",
    tag = "ui-theme",
    operation_id = "ui_theme_logo_get",
    responses(
        (status = 200, description = "Logo bytes (content-type as stored)"),
        (status = 404, description = "No logo configured"),
    ),
)]
pub(crate) async fn get_logo(state: Arc<ThemeState>) -> Response {
    match state.store.read_logo().await {
        Ok(Some((bytes, mime))) => bytes_response(bytes, &mime),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => map_internal(e),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/ui/theme/favicon",
    tag = "ui-theme",
    operation_id = "ui_theme_favicon_get",
    responses(
        (status = 200, description = "Favicon bytes (content-type as stored)"),
        (status = 404, description = "No favicon configured"),
    ),
)]
pub(crate) async fn get_favicon(state: Arc<ThemeState>) -> Response {
    match state.store.read_favicon().await {
        Ok(Some((bytes, mime))) => bytes_response(bytes, &mime),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => map_internal(e),
    }
}

fn bytes_response(bytes: Vec<u8>, content_type: &str) -> Response {
    HttpResponse::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .expect("static headers are valid")
}
