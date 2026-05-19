//! Create a new session row and return the cookie value.

use super::store::SessionStore;

/// Issue a session for `user_id` and return the opaque session id
/// the cookie carries.
///
/// Stubbed for v0.1.
pub async fn issue(_store: &SessionStore, _user_id: &str) -> Result<String, SessionError> {
    todo!("issue impl lands with the auth migrations")
}

/// Session-handling failures.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// Database error.
    #[error("session store error")]
    Store,
    /// Session not found or expired.
    #[error("session not found")]
    NotFound,
}
