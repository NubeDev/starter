//! Mint a new session row for `user_id`.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use rand::RngCore;

use crate::store::{SessionStore, SessionStoreError};

/// Prefix on session-cookie values, mirroring the `sak_` API-token
/// prefix. Makes it easy for the `Authenticator` to route a presented
/// credential without an extra table lookup.
pub const SESSION_PREFIX: &str = "sas_";

/// Default session lifetime — 24 h, in line with most browser-app
/// expectations. Consumers wanting a different value override at the
/// builder layer.
pub const DEFAULT_TTL_HOURS: i64 = 24;

/// Result of [`issue`] — the cookie value the browser stores and the
/// CSRF double-submit token the client must echo back on mutating
/// requests.
#[derive(Debug, Clone)]
pub struct IssuedSession {
    /// Value to set on the session cookie (`sas_<random>`).
    pub cookie_value: String,
    /// CSRF token to set on a non-httpOnly cookie and require back as
    /// the `X-CSRF-Token` header on mutating cookie requests.
    pub csrf_token: String,
}

/// Issue a session for `user_id`. Generates a fresh opaque id +
/// CSRF token and persists both.
pub async fn issue<S: SessionStore + ?Sized>(
    store: &S,
    user_id: &str,
) -> Result<IssuedSession, SessionError> {
    let mut id_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut id_bytes);
    let id = format!("{SESSION_PREFIX}{}", URL_SAFE_NO_PAD.encode(id_bytes));

    let mut csrf_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut csrf_bytes);
    let csrf_token = URL_SAFE_NO_PAD.encode(csrf_bytes);

    let expires_at = Utc::now() + Duration::hours(DEFAULT_TTL_HOURS);
    store
        .create(&id, user_id, &csrf_token, expires_at)
        .await
        .map_err(|e| match e {
            SessionStoreError::Backend(s) => SessionError::Store(s),
        })?;

    Ok(IssuedSession {
        cookie_value: id,
        csrf_token,
    })
}

/// Session-handling failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SessionError {
    /// Database error.
    #[error("session store error: {0}")]
    Store(String),
    /// Session not found or expired.
    #[error("session not found")]
    NotFound,
    /// CSRF token missing or mismatched.
    #[error("csrf token mismatch")]
    CsrfMismatch,
}
