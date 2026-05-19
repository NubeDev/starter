//! The `Authenticator` trait. Implementations wrap Zitadel, Clerk,
//! a local JWT verifier, an mTLS check — whatever the consumer
//! deploys.

use async_trait::async_trait;

use crate::error::Result;

use super::principal::Principal;

/// Verifies an opaque credential string and produces a [`Principal`].
///
/// Implementations are expected to be cheap (cached JWKS, etc.) —
/// `verify` is on the hot path of every authenticated request.
///
/// # Signature rationale (SCOPE open question 3)
///
/// `verify(&str)` is deliberately transport-agnostic. Two alternatives
/// were considered and rejected:
///
/// 1. **`verify(&http::request::Parts)`** — pulls all of `http` into
///    `starter-spi` (currently zero web-framework deps), and couples
///    every `Authenticator` impl to HTTP. The MCP stdio dispatcher
///    has no `Parts`; it would need to forge one to call into the
///    trait.
/// 2. **`verify(&AuthContext)` with pre-parsed cookie + bearer** — moves
///    parsing inward but still bakes HTTP cookies into the contract.
///    Worse, it forces every transport (HTTP, MCP, future gRPC) to
///    materialise an `AuthContext` even when it only carries one
///    credential kind.
///
/// `&str` keeps the trait useful from any transport. The HTTP boundary
/// ([`starter_server::auth::with_principal`]) pre-parses
/// `Authorization: Bearer …` and the `starter_session` cookie into a
/// single string. The auth-users bridge then routes on string prefix
/// (`sak_` → API token, `sas_` → session) — no `Parts` access needed.
/// External providers (Zitadel/Clerk) likewise receive the raw JWT.
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
