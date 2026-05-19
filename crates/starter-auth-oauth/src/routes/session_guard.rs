//! Session + CSRF gating shared by the link / unlink / identities
//! handlers. Phase 3 (stage 8) routes are normal `POST` / `DELETE`
//! against a logged-in browser, so they use the same double-submit
//! cookie shape `starter_auth_users::routes::logout` enforces — only
//! the OAuth `GET /callback` is allowed to skip CSRF (Hard rule R9,
//! and the OAuth `state` parameter substitutes there).
//!
//! The helper resolves the session cookie into a user id by joining
//! [`SessionStore`] with [`UserStore`] — every endpoint that consumes
//! it needs the user id, not a richer `Principal`, because authorisation
//! is implicit (the user can only touch their own identities). Anything
//! richer would invite drift with `starter-auth-users`'s `Principal`.

use std::collections::HashMap;

use axum::http::header::{HeaderMap, COOKIE};
use axum::http::StatusCode;
use starter_auth_users::session::{verify_session, SessionError, SESSION_COOKIE};
use starter_auth_users::store::{SessionStore, UserStore};

use crate::session_bridge::CSRF_COOKIE;

/// Outcome of the session + CSRF check. The error half is the HTTP
/// status the route returns directly; the success half is the
/// resolved user id.
pub(super) enum GuardOutcome {
    /// Caller has a valid session **and** echoed the CSRF token. The
    /// inner string is the resolved `user_id`.
    Allow(String),
    /// Caller failed the check; the status is the response the
    /// handler returns verbatim.
    Deny(StatusCode),
}

/// Verify a session cookie plus the double-submit CSRF header.
///
/// Returns [`GuardOutcome::Allow(user_id)`] on success. The failure
/// modes map deliberately to the same status codes the password
/// `/auth/logout` route uses so an SPA does not have to special-case
/// OAuth-side errors.
pub(super) async fn require_session_csrf(
    sessions: &dyn SessionStore,
    users: &dyn UserStore,
    headers: &HeaderMap,
    enforce_csrf: bool,
) -> GuardOutcome {
    let cookies = parse_cookies(headers);

    let session_value = match cookies.get(SESSION_COOKIE) {
        Some(v) => v.clone(),
        None => return GuardOutcome::Deny(StatusCode::UNAUTHORIZED),
    };

    if enforce_csrf {
        let cookie_csrf = cookies.get(CSRF_COOKIE).cloned();
        let header_csrf = headers
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        match (cookie_csrf, header_csrf) {
            (Some(c), Some(h)) if c == h => {}
            _ => return GuardOutcome::Deny(StatusCode::FORBIDDEN),
        }
    }

    match verify_session(sessions, users, &session_value).await {
        Ok(principal) => GuardOutcome::Allow(principal.subject),
        Err(SessionError::NotFound) => GuardOutcome::Deny(StatusCode::UNAUTHORIZED),
        Err(_) => GuardOutcome::Deny(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Parse the `Cookie` header(s) into a map. Mirrors the helper in
/// `starter_auth_users::routes::logout` byte-for-byte so a future
/// refactor can hoist both into a shared crate without behaviour
/// drift.
fn parse_cookies(headers: &HeaderMap) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for value in headers.get_all(COOKIE) {
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
