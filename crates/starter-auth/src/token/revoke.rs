//! Revoke a token by its row id.

use super::issue::TokenError;

/// Mark a token row revoked. Idempotent — revoking an already-revoked
/// token is not an error.
pub async fn revoke(_token_id: &str) -> Result<(), TokenError> {
    todo!("revoke impl lands with the auth migrations")
}
