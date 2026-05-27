//! `GET /auth/me`. Returns the current user's identity or 401.
//! UI calls this on mount to discover whether the user is logged in.
//!
//! Accepts either credential the agent issues:
//! - `Authorization: Bearer sak_…` API token (issued by `/auth/token`)
//! - `SESSION_COOKIE=sas_…` session cookie (issued by `/auth/login`)
//!
//! Bearer takes precedence so machine clients that don't keep cookies
//! (Flutter web, CLI, server-to-server) work without a cookie jar.

use std::sync::Arc;

use axum::http::header::{HeaderMap, AUTHORIZATION};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::session::verify_session;
use crate::token::{verify as verify_token, TOKEN_PREFIX};

use super::state::AuthState;

/// Response body for `GET /auth/me`.
#[derive(Debug, Serialize, ToSchema)]
pub struct MeResponse {
    /// Stable user identifier (the `Principal.subject`).
    pub subject: String,
    /// User's email.
    pub email: String,
    /// Role: reader / writer / admin.
    pub role: crate::role::Role,
}

#[utoipa::path(
    get,
    path = "/auth/me",
    tag = "auth",
    operation_id = "me",
    responses(
        (status = 200, description = "Current user identity", body = MeResponse),
        (status = 401, description = "Not authenticated"),
    ),
)]
pub(crate) async fn handler(state: Arc<AuthState>, headers: HeaderMap) -> Response {
    // Bearer first — machine clients (Flutter, CLI) authenticate this way.
    if let Some(token) = bearer_token(&headers) {
        return match verify_token(state.tokens.as_ref(), state.users.as_ref(), &token).await {
            Ok(principal) => match state.users.find_by_id(&principal.subject).await {
                Ok(Some(user)) => Json(MeResponse {
                    subject: principal.subject,
                    email: user.email,
                    role: principal.role,
                })
                .into_response(),
                _ => StatusCode::UNAUTHORIZED.into_response(),
            },
            Err(_) => StatusCode::UNAUTHORIZED.into_response(),
        };
    }

    let cookie_value = match cookie_value(&headers, crate::session::SESSION_COOKIE) {
        Some(v) => v,
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let principal =
        match verify_session(state.sessions.as_ref(), state.users.as_ref(), &cookie_value).await {
            Ok(p) => p,
            Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
        };

    let user = match state.users.find_by_id(&principal.subject).await {
        Ok(Some(u)) => u,
        _ => return StatusCode::UNAUTHORIZED.into_response(),
    };

    Json(MeResponse {
        subject: principal.subject,
        email: user.email,
        role: principal.role,
    })
    .into_response()
}

/// Extract an API-token bearer (`sak_…`) from the `Authorization` header.
/// Returns `None` for missing, malformed, or non-token (e.g. `sas_…`,
/// JWT) values so the cookie path can take over.
fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = raw.strip_prefix("Bearer ").or_else(|| raw.strip_prefix("bearer "))?;
    let token = token.trim();
    if token.starts_with(TOKEN_PREFIX) {
        Some(token.to_string())
    } else {
        None
    }
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    for value in headers.get_all(axum::http::header::COOKIE) {
        if let Ok(s) = value.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    if k == name {
                        return Some(v.to_string());
                    }
                }
            }
        }
    }
    None
}
