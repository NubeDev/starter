//! `PUT /api/v1/ui/theme` — replace the styles + shell.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use starter_spi::ui::theme::{validate_save_input, ThemeDocument, ThemeSaveInput};

use super::errors::{map_internal, map_token_error, problem};
use super::guards::require_admin;
use super::state::ThemeState;

/// Maximum JSON body size for a `PUT /api/v1/ui/theme`. 64 KiB is
/// generous for 38 tokens × 2 modes plus the shell sidecar; the cap
/// stops a malicious client streaming arbitrary JSON through us.
const MAX_BODY: usize = 64 * 1024;

#[utoipa::path(
    put,
    path = "/api/v1/ui/theme",
    tag = "ui-theme",
    operation_id = "ui_theme_put",
    request_body = ThemeSaveInput,
    responses(
        (status = 200, description = "Saved; returns the updated document", body = ThemeDocument),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin role required"),
        (status = 413, description = "Body too large"),
    ),
)]
pub(crate) async fn put_theme(state: Arc<ThemeState>, req: Request<Body>) -> Response {
    if let Some(resp) = require_admin(&req) {
        return resp;
    }
    let (_, body) = req.into_parts();
    let bytes = match to_bytes(body, MAX_BODY).await {
        Ok(b) => b,
        Err(_) => {
            return problem(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "request body exceeds 64 KiB",
                None,
            );
        }
    };
    let input: ThemeSaveInput = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return problem(
                StatusCode::BAD_REQUEST,
                "invalid_input",
                "could not parse theme document",
                Some(e.to_string()),
            );
        }
    };
    if let Err(err) = validate_save_input(&input) {
        return map_token_error(err);
    }
    match state.store.save(input).await {
        Ok(doc) => (StatusCode::OK, Json(doc)).into_response(),
        Err(e) => map_internal(e),
    }
}
