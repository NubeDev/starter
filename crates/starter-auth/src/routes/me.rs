//! `GET /auth/me`. Returns the current `Principal` or 401.
//! UI calls this on mount to discover whether the user is logged in.

use serde::Serialize;

/// Response body for `GET /auth/me`.
#[derive(Debug, Serialize)]
pub struct MeResponse {
    /// Stable user identifier (the `Principal.subject`).
    pub subject: String,
    /// User's email.
    pub email: String,
    /// Role: reader / writer / admin.
    pub role: crate::role::Role,
}

// TODO(ap): handler body lands with the authenticator impl.
