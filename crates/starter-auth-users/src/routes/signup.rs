//! `POST /auth/signup`. Body: `{ email, password }`. On success: sets
//! the session cookie + a non-httpOnly CSRF cookie (same shape as
//! login), returns the CSRF token in the response body.

use std::net::IpAddr;
use std::sync::Arc;

use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::password;
use crate::role::Role;
use crate::session::{issue, IssuedSession, SESSION_COOKIE};
use crate::signup::validate::{self, ValidationError};
use crate::store::UserStoreError;

use super::login::CSRF_COOKIE;
use super::state::AuthState;

/// Request body for `POST /auth/signup`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SignupRequest {
    /// Email address for the new account.
    pub email: String,
    /// Plaintext password (min 12 chars by default).
    pub password: String,
}

/// Response body for `POST /auth/signup` on success.
#[derive(Debug, Serialize, ToSchema)]
pub struct SignupResponse {
    /// CSRF double-submit token. Send back as `X-CSRF-Token` on
    /// mutating cookie-authenticated requests.
    pub csrf_token: String,
}

/// Error body for signup failures (400, 409, 429).
#[derive(Debug, Serialize, ToSchema)]
pub struct SignupError {
    /// Machine-readable error code.
    pub error: String,
    /// Human-readable explanation (optional, absent on 409/429).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[utoipa::path(
    post,
    path = "/auth/signup",
    tag = "auth",
    operation_id = "signup",
    request_body = SignupRequest,
    responses(
        (status = 200, description = "Signed up; session + CSRF cookies set", body = SignupResponse),
        (status = 400, description = "Validation error", body = SignupError),
        (status = 409, description = "Email already registered", body = SignupError),
        (status = 429, description = "Rate limited", body = SignupError),
    ),
)]
pub(crate) async fn handler(
    state: Arc<AuthState>,
    ip: IpAddr,
    default_role: Role,
    Json(body): Json<SignupRequest>,
) -> Response {
    let email_normalised = body.email.trim().to_lowercase();

    // R6: rate-limit check BEFORE any DB work and BEFORE password
    // hashing — Argon2id is deliberately expensive, so we must not
    // let a flood of requests burn CPU.
    if let Err(limited) = state.rate_limit.check(ip, &email_normalised).await {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            Json(SignupError {
                error: "rate_limited".into(),
                message: None,
            }),
        )
            .into_response();
        if let Ok(v) = HeaderValue::from_str(&limited.retry_after_secs.to_string()) {
            resp.headers_mut().insert("retry-after", v);
        }
        return resp;
    }

    // Validate email + password.
    let min_len = validate::password_min_len_from_env();
    if let Err(e) = validate::validate_signup_input(&email_normalised, &body.password, min_len) {
        let (error, message) = match e {
            ValidationError::InvalidEmail(msg) => ("invalid_email".to_owned(), msg),
            ValidationError::WeakPassword(msg) => ("weak_password".to_owned(), msg),
        };
        return (
            StatusCode::BAD_REQUEST,
            Json(SignupError {
                error,
                message: Some(message),
            }),
        )
            .into_response();
    }

    // Hash password.
    let hash = match password::hash(&body.password) {
        Ok(h) => h,
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "signup password hash failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Insert user.
    let user_id = Uuid::new_v4().to_string();
    if let Err(e) = state
        .users
        .create(&user_id, &email_normalised, Some(&hash), default_role)
        .await
    {
        return match e {
            // R4: uniform 409 regardless of whether the existing
            // account is password-based or OAuth-only.
            UserStoreError::Conflict => (
                StatusCode::CONFLICT,
                Json(SignupError {
                    error: "email_already_registered".into(),
                    message: None,
                }),
            )
                .into_response(),
            _ => {
                tracing::warn!(target: "starter_auth_users", error = %e, "signup user create failed");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        };
    }

    // email_verified = false for signup-created users (R7).
    if let Err(e) = state.users.set_email_verified(&user_id, false).await {
        tracing::warn!(target: "starter_auth_users", error = %e, "signup set_email_verified failed");
    }

    // Mint session — same path as login (R1).
    let issued: IssuedSession = match issue(state.sessions.as_ref(), &user_id).await {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "signup session issue failed");
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
        Json(SignupResponse {
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
