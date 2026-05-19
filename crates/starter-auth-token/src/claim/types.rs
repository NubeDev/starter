//! Public types surfaced by the claim flow.

use serde::{Deserialize, Serialize};

/// The pending claim token surfaced on first boot. Holds the
/// plaintext value once; subsequent reads of the same row return
/// only the row id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingToken {
    /// Row id (also the `claim_id` that becomes the `Principal.subject`
    /// after a successful claim).
    pub id: String,
    /// Plaintext token the operator presents on `POST /auth/claim`.
    /// 32 random bytes, base64url-no-pad encoded.
    pub plaintext: String,
}

/// The owner token issued by a successful claim. Plaintext is shown
/// to the caller exactly once; the database stores only the SHA-256
/// digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedToken {
    /// Row id — becomes `Principal.subject` for every bearer-authed
    /// request.
    pub claim_id: String,
    /// Plaintext owner token. Show, then forget.
    pub plaintext: String,
}

/// Errors specific to the claim flow.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    /// No pending row exists. Either the server has already been
    /// claimed (call [`crate::regenerate_claim_pending`] to reset)
    /// or first-boot bookkeeping has not run.
    #[error("no pending claim token")]
    NoPending,

    /// A claim has already been performed. Resetting requires the
    /// out-of-band regenerate path; the HTTP surface does not allow
    /// silent re-claim.
    #[error("already claimed")]
    AlreadyClaimed,

    /// Presented token did not match the pending row.
    #[error("invalid claim token")]
    InvalidToken,

    /// Backing store failed.
    #[error("claim store error: {0}")]
    Store(String),
}
