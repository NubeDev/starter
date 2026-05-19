//! # starter-auth-token
//!
//! Single-owner `Authenticator`. Designed for headless / appliance
//! deployments where there is exactly one operator and no concept of
//! multi-user login.
//!
//! Lifecycle (SCOPE 444–492):
//!
//! 1. On first boot the server generates a 32-byte base64url
//!    `claim_token`. It lives in `starter_auth_token_pending` and is
//!    surfaced once — typically via logs and/or a sibling
//!    `SecretStore` entry at key `auth-token:pending`.
//! 2. The operator hits `POST /auth/claim` carrying that token. The
//!    server consumes the pending row, generates a fresh
//!    `owner_token`, stores its SHA-256 digest in
//!    `starter_auth_token_claimed`, and returns the plaintext
//!    `owner_token` exactly once.
//! 3. From then on every request must present
//!    `Authorization: Bearer <owner_token>`. The
//!    [`TokenAuthenticator`] does a constant-time SHA-256 comparison
//!    against the stored digest and yields
//!    `Principal { subject: claim_id, role: Admin, scopes: vec![] }`.
//! 4. Factory reset via [`regenerate_claim_pending`] wipes the
//!    claimed row, bumps the auth epoch (invalidating any cached
//!    bearer), and re-issues a fresh pending token.
//!
//! Mutually exclusive with `starter-auth-users` (SCOPE 228–232) —
//! the consumer wires one or the other.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod authenticator;
pub mod claim;
pub mod routes;
pub mod store;

pub use authenticator::TokenAuthenticator;
pub use claim::{
    claim_pending, regenerate_claim_pending, regenerate_claim_pending_with_secrets, ClaimError,
    ClaimedToken, PendingToken, PENDING_SECRET_KEY,
};
pub use store::ClaimStore;
