//! Verify a presented bearer token against the hashed-token table
//! and return the matching principal.

use starter_spi::auth::Principal;

use super::issue::TokenError;

/// Verify `presented` against the token table.
///
/// Returns the principal owning the token on success. Updates
/// `last_used_at` as a side effect (best-effort — failures are
/// logged but not surfaced).
pub async fn verify(_presented: &str) -> Result<Principal, TokenError> {
    todo!("verify impl lands with the auth migrations")
}
