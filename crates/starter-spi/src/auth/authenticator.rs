//! The `Authenticator` trait. Implementations wrap Zitadel, Clerk,
//! a local JWT verifier, an mTLS check — whatever the consumer
//! deploys.

use async_trait::async_trait;

use crate::error::Result;

use super::principal::Principal;

/// Verifies a bearer credential and produces a [`Principal`].
///
/// Implementations are expected to be cheap (cached JWKS, etc.) —
/// `verify` is on the hot path of every authenticated request.
#[async_trait]
pub trait Authenticator: Send + Sync + 'static {
    /// Verify the raw credential the transport extracted (e.g.
    /// the value after `Bearer ` in the `Authorization` header).
    ///
    /// Returns `Error::Unauthenticated` for malformed / expired
    /// credentials, or `Error::Internal` for downstream failures
    /// (JWKS fetch failed, etc.).
    async fn verify(&self, credential: &str) -> Result<Principal>;
}
