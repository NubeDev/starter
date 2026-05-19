//! `regenerate_claim_pending` — factory-reset path. Wipes any claimed
//! row, bumps the auth epoch (so cached bearers stop working), and
//! issues a fresh pending token.
//!
//! Exposed via `starter-cli reset --force` per SCOPE 473–476.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use starter_spi::secrets::{Secret, SecretStore};

use super::types::{ClaimError, PendingToken};
use crate::store::ClaimStore;

/// Key under which the freshly-generated pending plaintext is written
/// when a `SecretStore` is supplied to
/// [`regenerate_claim_pending_with_secrets`].
pub const PENDING_SECRET_KEY: &str = "auth-token:pending";

/// Generate 32 fresh random bytes, base64url-no-pad encode them,
/// and store them as the new pending claim.
///
/// Returns the plaintext to surface to the operator. The previous
/// claimed and pending rows (if any) are removed in the same
/// transaction; the auth epoch is bumped so any cached bearer
/// derived from the prior claim stops authenticating.
pub async fn regenerate_claim_pending<S: ClaimStore + ?Sized>(
    store: &S,
) -> Result<PendingToken, ClaimError> {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let plaintext = URL_SAFE_NO_PAD.encode(bytes);
    let id = store.reset_with_new_pending(&plaintext).await?;
    Ok(PendingToken { id, plaintext })
}

/// Same as [`regenerate_claim_pending`], but also writes the freshly
/// generated plaintext to `secrets` at key
/// [`PENDING_SECRET_KEY`] (`auth-token:pending`) so an operator can
/// read it back through the configured `SecretStore` rather than
/// only from the log line (SCOPE 488–492).
///
/// A failure to write the secret is mapped to `ClaimError::Store` so
/// the caller sees one error type. The database row has already been
/// committed by that point; on `Err` the operator can still recover
/// via the log line or by calling the regenerate path again.
pub async fn regenerate_claim_pending_with_secrets<S, T>(
    store: &S,
    secrets: &T,
) -> Result<PendingToken, ClaimError>
where
    S: ClaimStore + ?Sized,
    T: SecretStore + ?Sized,
{
    let pending = regenerate_claim_pending(store).await?;
    secrets
        .put(PENDING_SECRET_KEY, Secret::new(pending.plaintext.clone()))
        .map_err(|e| ClaimError::Store(e.to_string()))?;
    Ok(pending)
}
