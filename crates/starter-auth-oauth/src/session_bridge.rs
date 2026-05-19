//! Bridge from a resolved OAuth identity to a `starter-auth-users`
//! session cookie.
//!
//! Per Hard rule R1, OAuth ends in a `sas_*` session — the same
//! credential `POST /auth/login` mints. This module is the
//! one-function boundary that calls into
//! [`starter_auth_users::session::issue`] and packages the result
//! as `Set-Cookie` headers ready to attach to a `302 Found`
//! response.
//!
//! Centralised so the two callback paths (sign-in and link-mode,
//! both landing in `routes/callback.rs`) emit the *same* cookie
//! shape the password login emits; a downstream check like
//! `verify_session` cannot tell which path minted the credential.

use std::sync::Arc;

use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::http::HeaderMap;
use starter_auth_users::session::{issue, IssuedSession, SESSION_COOKIE};
use starter_auth_users::store::SessionStore;

/// Cookie name carrying the CSRF double-submit token, kept aligned
/// with `starter_auth_users::routes::login::CSRF_COOKIE` — re-declared
/// here so this crate does not import a `pub(crate)` constant. If the
/// users crate ever exposes it under `pub`, replace this with a
/// re-export.
pub const CSRF_COOKIE: &str = "starter_csrf";

/// Mint a session for `user_id` and return the `Set-Cookie` headers
/// the callback handler appends to its `302` response.
///
/// `Path=/` + `SameSite=Lax` mirror the password login; `HttpOnly`
/// on the session cookie keeps the opaque id out of `document.cookie`
/// while the CSRF cookie stays JS-readable so an SPA can echo it
/// back as `X-CSRF-Token`.
pub async fn mint_session_headers(
    sessions: Arc<dyn SessionStore>,
    user_id: &str,
) -> Result<HeaderMap, starter_auth_users::session::SessionError> {
    let issued: IssuedSession = issue(sessions.as_ref(), user_id).await?;

    let session_cookie = format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax",
        value = issued.cookie_value,
    );
    let csrf_cookie = format!(
        "{CSRF_COOKIE}={value}; Path=/; SameSite=Lax",
        value = issued.csrf_token,
    );

    let mut headers = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&session_cookie) {
        headers.append(SET_COOKIE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&csrf_cookie) {
        headers.append(SET_COOKIE, v);
    }
    Ok(headers)
}
