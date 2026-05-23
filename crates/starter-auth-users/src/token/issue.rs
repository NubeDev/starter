//! Issue a new API token. Returns the plaintext token exactly once
//! — the database stores only the argon2id hash of the secret half.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, Utc};
use rand::RngCore;

use crate::password::{hash, PasswordError};
use crate::scope::Scope;
use crate::store::{TokenStore, TokenStoreError};

/// Prefix on every API token plaintext: `sak_` (starter-auth-keys).
/// Mirrors the session prefix so the `Authenticator` routes by string
/// prefix without an extra DB lookup.
pub const TOKEN_PREFIX: &str = "sak_";

/// Result of [`issue`].
#[derive(Debug, Clone)]
pub struct IssuedToken {
    /// Public lookup id (the segment between the `sak_` prefix and the
    /// `.` separator). Lives in the database in cleartext.
    pub id: String,
    /// Full plaintext token: `sak_<id>.<secret>`. Show once, then
    /// forget — only the argon2-hashed secret is persisted.
    pub plaintext: String,
}

/// The super-admin sentinel `Principal.tenant_id` / token tenant
/// binding (Phase 7a — R11). Tokens minted with this value bypass
/// the cross-tenant predicate and are only allowed for users with
/// the global Admin role.
pub const SUPER_ADMIN_TENANT: &str = "*";

/// Issue a new token for `(user_id, tenant_id)` with the given
/// scopes and optional absolute expiry.
///
/// Phase 7a (R11): every token is bound to exactly one tenant.
/// Pass `tenant_id = "*"` (the [`SUPER_ADMIN_TENANT`] sentinel)
/// only for users your caller has verified hold the global Admin
/// role — that binding bypasses the cross-tenant predicate. The
/// callers in `routes/tokens.rs` perform that check before
/// calling.
pub async fn issue<S: TokenStore + ?Sized>(
    store: &S,
    user_id: &str,
    scopes: &[Scope],
    tenant_id: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<IssuedToken, TokenError> {
    let mut id_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut id_bytes);
    let id = URL_SAFE_NO_PAD.encode(id_bytes);

    let mut secret_bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut secret_bytes);
    let secret = URL_SAFE_NO_PAD.encode(secret_bytes);

    let plaintext = format!("{TOKEN_PREFIX}{id}.{secret}");
    let hashed = hash(&secret).map_err(|_: PasswordError| TokenError::Internal)?;

    store
        .create(&id, user_id, &hashed, scopes, tenant_id, expires_at)
        .await
        .map_err(map_store_err)?;

    Ok(IssuedToken { id, plaintext })
}

/// Token-handling failures.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TokenError {
    /// Database error.
    #[error("token store error: {0}")]
    Store(String),
    /// Presented token did not match any row or did not verify.
    #[error("invalid token")]
    Invalid,
    /// Token row was revoked or expired.
    #[error("token revoked or expired")]
    Revoked,
    /// Hashing failed.
    #[error("token hashing failed")]
    Internal,
}

pub(crate) fn map_store_err(e: TokenStoreError) -> TokenError {
    match e {
        TokenStoreError::Backend(s) | TokenStoreError::BadScopes(s) => TokenError::Store(s),
    }
}
