//! Revoke a token by its row id.

use crate::store::TokenStore;

use super::issue::{map_store_err, TokenError};

/// Mark a token row revoked. Idempotent — revoking an already-revoked
/// token is not an error.
pub async fn revoke<T: TokenStore + ?Sized>(store: &T, token_id: &str) -> Result<(), TokenError> {
    store.revoke(token_id).await.map_err(map_store_err)
}
