//! `POST /auth/logout`. Reads the session cookie, revokes the row,
//! clears the cookie. Returns 204 even if the cookie was missing
//! (idempotent).
//!
//! Enforces CSRF: the caller must echo back the cookie's CSRF token
//! as `X-CSRF-Token`. Missing / mismatched header → 403.

use std::sync::Arc;

use axum::http::header::{HeaderMap, HeaderValue, SET_COOKIE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::session::{revoke, SESSION_COOKIE};

use super::login::CSRF_COOKIE;
use super::state::AuthState;

pub(super) async fn handler(state: Arc<AuthState>, headers: HeaderMap) -> Response {
    let cookies = parse_cookies(&headers);
    let session_id = match cookies.get(SESSION_COOKIE) {
        Some(s) => s.to_string(),
        None => return clear_cookies(StatusCode::NO_CONTENT.into_response()),
    };

    let cookie_csrf = cookies.get(CSRF_COOKIE).map(|s| s.to_string());
    let header_csrf = headers
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    match (cookie_csrf, header_csrf) {
        (Some(c), Some(h)) if c == h => {}
        _ => return StatusCode::FORBIDDEN.into_response(),
    }

    if let Err(e) = revoke(state.sessions.as_ref(), &session_id).await {
        tracing::warn!(target: "starter_auth_users", error = %e, "session revoke failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }
    clear_cookies(StatusCode::NO_CONTENT.into_response())
}

fn parse_cookies(headers: &HeaderMap) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for value in headers.get_all(axum::http::header::COOKIE) {
        if let Ok(s) = value.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    out.insert(k.to_string(), v.to_string());
                }
            }
        }
    }
    out
}

fn clear_cookies(mut resp: Response) -> Response {
    let headers = resp.headers_mut();
    let expired = format!("{SESSION_COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",);
    let expired_csrf = format!("{CSRF_COOKIE}=; Path=/; SameSite=Lax; Max-Age=0");
    if let Ok(v) = HeaderValue::from_str(&expired) {
        headers.append(SET_COOKIE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&expired_csrf) {
        headers.append(SET_COOKIE, v);
    }
    resp
}
