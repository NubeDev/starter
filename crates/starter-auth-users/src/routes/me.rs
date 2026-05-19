//! `GET /auth/me`. Returns the current user's identity or 401.
//! UI calls this on mount to discover whether the user is logged in.

use std::sync::Arc;

use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::session::verify_session;

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
