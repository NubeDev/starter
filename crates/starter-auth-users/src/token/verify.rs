//! Verify a presented bearer token against the hashed-token table
//! and return the matching principal.

use starter_spi::auth::Principal;

use crate::password;
use crate::store::{TokenStore, UserStore, UserStoreError};

use super::issue::{map_store_err, TokenError, TOKEN_PREFIX};

/// Verify `presented` (the full `sak_<id>.<secret>` plaintext) against
/// the token table.
///
/// On success, returns the principal owning the token and updates
/// `last_used_at` as a side effect — failures of the touch are
/// logged but do not fail the request.
pub async fn verify<T, U>(tokens: &T, users: &U, presented: &str) -> Result<Principal, TokenError>
where
    T: TokenStore + ?Sized,
    U: UserStore + ?Sized,
{
    let rest = presented
        .strip_prefix(TOKEN_PREFIX)
        .ok_or(TokenError::Invalid)?;
    let (id, secret) = rest.split_once('.').ok_or(TokenError::Invalid)?;

    let row = tokens
        .find_active(id)
        .await
        .map_err(map_store_err)?
        .ok_or(TokenError::Revoked)?;

    match password::verify(secret, &row.hashed_token) {
        Ok(true) => {}
        Ok(false) => return Err(TokenError::Invalid),
        Err(_) => return Err(TokenError::Internal),
    }

    if let Err(e) = tokens.touch_last_used(&row.id).await {
        tracing::warn!(
            target: "starter_auth_users",
            error = %e,
            "failed to update token last_used_at",
        );
    }

    let user = users
        .find_by_id(&row.user_id)
        .await
        .map_err(|e| match e {
            UserStoreError::Backend(s) => TokenError::Store(s),
            UserStoreError::NotFound | UserStoreError::Conflict => TokenError::Invalid,
        })?
        .ok_or(TokenError::Invalid)?;

    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: row.scopes,
        extra: serde_json::Value::Null,
    })
}
