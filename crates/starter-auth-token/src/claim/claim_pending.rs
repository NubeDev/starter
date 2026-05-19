//! `claim_pending` — consume the pending row, issue + persist the
//! owner token's digest. The single seam called by
//! `POST /auth/claim`.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::types::{ClaimError, ClaimedToken};
use crate::store::ClaimStore;

/// Verify `presented` against the pending row in constant time and,
/// on a match, generate the owner token, store its SHA-256 digest,
/// and return the plaintext.
///
/// On second-attempt re-claim returns [`ClaimError::AlreadyClaimed`]
/// (the store will have moved past the pending state).
pub async fn claim_pending<S: ClaimStore + ?Sized>(
    store: &S,
    presented: &str,
) -> Result<ClaimedToken, ClaimError> {
    if store.is_claimed().await? {
        return Err(ClaimError::AlreadyClaimed);
    }
    let pending = store.fetch_pending().await?.ok_or(ClaimError::NoPending)?;
    let presented_bytes = presented.as_bytes();
    let stored_bytes = pending.plaintext.as_bytes();
    let matches: bool = presented_bytes.ct_eq(stored_bytes).into();
    if !matches {
        return Err(ClaimError::InvalidToken);
    }

    let mut owner_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut owner_bytes);
    let owner_plaintext = URL_SAFE_NO_PAD.encode(owner_bytes);
    let digest: [u8; 32] = Sha256::digest(owner_plaintext.as_bytes()).into();

    store.promote_to_claimed(&pending.id, &digest).await?;

    Ok(ClaimedToken {
        claim_id: pending.id,
        plaintext: owner_plaintext,
    })
}
