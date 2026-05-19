//! Issue a new API token. Returns the plaintext token exactly once
//! — the database stores only the argon2 hash.

/// Issue a new token for `user_id` with the given scopes.
///
/// Returns the plaintext token string. **This is the only time the
/// caller sees it** — losing it requires issuing a new one.
pub async fn issue(
    _user_id: &str,
    _scopes: &[crate::scope::Scope],
    _expires_at: Option<chrono::DateTime<chrono::Utc>>,
) -> Result<String, TokenError> {
    todo!("issue impl lands with the auth migrations")
}

/// Token-handling failures.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    /// Database error.
    #[error("token store error")]
    Store,
    /// Presented token did not match any row.
    #[error("invalid token")]
    Invalid,
    /// Token row was revoked or expired.
    #[error("token revoked or expired")]
    Revoked,
}
