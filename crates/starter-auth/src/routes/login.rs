//! `POST /auth/login`. Body: `{ email, password }`. On success:
//! sets the session cookie and returns 204.

use serde::Deserialize;

/// Request body for `POST /auth/login`.
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// User's email (the primary identifier in `starter_auth_users`).
    pub email: String,
    /// Plaintext password. Verified against the stored argon2 hash.
    pub password: String,
}

// TODO(ap): handler body. Public surface staged for axum
// `IntoResponse` once the session module lands.
