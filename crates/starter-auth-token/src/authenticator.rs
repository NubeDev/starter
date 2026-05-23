//! `TokenAuthenticator` — implements `starter_spi::auth::Authenticator`.
//!
//! Reads the claimed digest from the store and constant-time
//! compares it against the SHA-256 of the presented bearer.

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use starter_spi::{
    auth::{Authenticator, Principal, Role},
    error::Result,
    Error,
};
use subtle::ConstantTimeEq;

use crate::store::ClaimStore;

/// Single-owner authenticator. Wrap whichever [`ClaimStore`] impl
/// the consumer wired (sqlite / postgres).
pub struct TokenAuthenticator<S: ClaimStore> {
    store: S,
}

impl<S: ClaimStore> TokenAuthenticator<S> {
    /// Build the authenticator over `store`.
    pub fn new(store: S) -> Self {
        Self { store }
    }
}

#[async_trait]
impl<S: ClaimStore + 'static> Authenticator for TokenAuthenticator<S> {
    async fn verify(&self, credential: &str) -> Result<Principal> {
        let claimed = self
            .store
            .fetch_claimed_digest()
            .await
            .map_err(|e| Error::Internal {
                source: Box::new(e),
            })?
            .ok_or(Error::Unauthenticated)?;

        let presented_digest: [u8; 32] = Sha256::digest(credential.as_bytes()).into();
        let matches: bool = presented_digest.ct_eq(&claimed.digest).into();
        if !matches {
            return Err(Error::Unauthenticated);
        }

        Ok(Principal {
            subject: claimed.claim_id,
            role: Role::Admin,
            scopes: Vec::new(),
            tenant_id: None,
            extra: serde_json::Value::Null,
        })
    }
}
