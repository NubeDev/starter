//! `POST /api/v1/ui/theme/logo` + DELETE + favicon twins. Admin-only.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::{accepted_mime, limits};

use super::errors::{map_internal, problem};
use super::guards::require_admin;
use super::state::ThemeState;

fn content_type_str<B>(req: &Request<B>) -> Option<String> {
    req.headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        // Strip any `; charset=…` parameter — only the bare MIME
        // type participates in the allow list check.
        .map(|s| s.split(';').next().unwrap_or(s).trim().to_lowercase())
}

async fn upload_asset(
    state: Arc<ThemeState>,
    req: Request<Body>,
    accepted: &[&str],
    max_bytes: usize,
    asset: Asset,
) -> Response {
    if let Some(resp) = require_admin(&req) {
        return resp;
    }
    let content_type = match content_type_str(&req) {
        Some(s) if accepted.iter().any(|a| *a == s) => s,
        _ => {
            return problem(
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "unsupported_media_type",
                "unsupported asset content-type",
                Some(format!("accepted: {}", accepted.join(", "))),
            );
        }
    };
    let (_, body) = req.into_parts();
    let bytes = match to_bytes(body, max_bytes).await {
        Ok(b) => b,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "asset exceeds size limit",
                Some(format!("max {max_bytes} bytes")),
            );
        }
    };
    let result = match asset {
        Asset::Logo => state.store.put_logo(bytes.to_vec(), &content_type).await,
        Asset::Favicon => state.store.put_favicon(bytes.to_vec(), &content_type).await,
    };
    match result {
        Ok(_url) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_internal(e),
    }
}

async fn delete_asset(state: Arc<ThemeState>, req: Request<Body>, asset: Asset) -> Response {
    if let Some(resp) = require_admin(&req) {
        return resp;
    }
    let result = match asset {
        Asset::Logo => state.store.delete_logo().await,
        Asset::Favicon => state.store.delete_favicon().await,
    };
    match result {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => map_internal(e),
    }
}

#[derive(Copy, Clone)]
enum Asset {
    Logo,
    Favicon,
}

#[utoipa::path(
    post,
    path = "/api/v1/ui/theme/logo",
    tag = "ui-theme",
    operation_id = "ui_theme_logo_post",
    request_body(content = Vec<u8>, description = "Raw image bytes", content_type = "image/png"),
    responses(
        (status = 204, description = "Logo stored"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
        (status = 413, description = "Asset too large"),
        (status = 415, description = "Unsupported media type"),
    ),
)]
pub(crate) async fn post_logo(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    upload_asset(
        state,
        req,
        accepted_mime::LOGO,
        limits::LOGO_MAX_BYTES,
        Asset::Logo,
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/api/v1/ui/theme/logo",
    tag = "ui-theme",
    operation_id = "ui_theme_logo_delete",
    responses(
        (status = 204, description = "Logo cleared (idempotent)"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
    ),
)]
pub(crate) async fn delete_logo(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    delete_asset(state, req, Asset::Logo).await
}

#[utoipa::path(
    post,
    path = "/api/v1/ui/theme/favicon",
    tag = "ui-theme",
    operation_id = "ui_theme_favicon_post",
    request_body(content = Vec<u8>, description = "Raw image bytes", content_type = "image/png"),
    responses(
        (status = 204, description = "Favicon stored"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
        (status = 413, description = "Asset too large"),
        (status = 415, description = "Unsupported media type"),
    ),
)]
pub(crate) async fn post_favicon(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    upload_asset(
        state,
        req,
        accepted_mime::FAVICON,
        limits::FAVICON_MAX_BYTES,
        Asset::Favicon,
    )
    .await
}

#[utoipa::path(
    delete,
    path = "/api/v1/ui/theme/favicon",
    tag = "ui-theme",
    operation_id = "ui_theme_favicon_delete",
    responses(
        (status = 204, description = "Favicon cleared (idempotent)"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
    ),
)]
pub(crate) async fn delete_favicon(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    delete_asset(state, req, Asset::Favicon).await
}
