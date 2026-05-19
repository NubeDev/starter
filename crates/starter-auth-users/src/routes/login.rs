//! `POST /auth/login`. Body: `{ email, password }`. On success: sets
//! the session cookie + a non-httpOnly CSRF cookie, returns the CSRF
//! token in the response body so SPAs can pick it up without parsing
//! `Set-Cookie` headers.

use std::sync::Arc;

use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::password;
use crate::session::{issue, IssuedSession, SESSION_COOKIE};

use super::state::AuthState;

/// Cookie name carrying the CSRF double-submit token. Non-httpOnly on
/// purpose so the SPA can read it and echo it back as
/// `X-CSRF-Token`.
pub const CSRF_COOKIE: &str = "starter_csrf";

/// Request body for `POST /auth/login`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// User's email (the primary identifier in `starter_auth_users`).
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Response body for `POST /auth/login`.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// CSRF double-submit token. Send back as `X-CSRF-Token` on
    /// mutating cookie-authenticated requests.
    pub csrf_token: String,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    operation_id = "login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in; session + CSRF cookies set", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
    ),
)]
pub(crate) async fn handler(state: Arc<AuthState>, Json(body): Json<LoginRequest>) -> Response {
    let user = match state.users.find_by_email(&body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "login store lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    match password::verify(&body.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "login password verify failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    let issued: IssuedSession = match issue(state.sessions.as_ref(), &user.id).await {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "session issue failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let session_cookie = format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax",
        value = issued.cookie_value,
    );
    let csrf_cookie = format!(
        "{CSRF_COOKIE}={value}; Path=/; SameSite=Lax",
        value = issued.csrf_token,
    );

    let mut resp = (
        StatusCode::OK,
        Json(LoginResponse {
            csrf_token: issued.csrf_token.clone(),
        }),
    )
        .into_response();
    let headers = resp.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&session_cookie) {
        headers.append(SET_COOKIE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&csrf_cookie) {
        headers.append(SET_COOKIE, v);
    }
    resp
}
