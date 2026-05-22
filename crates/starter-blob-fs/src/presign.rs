//! Presign HMAC keyed by a caller-supplied [`PresignKey`].
//!
//! # Why the key is mandatory at construction
//!
//! The fs engine stores bytes durably across restarts; the
//! presigned-URL contract has to match. If the engine generated a
//! fresh HMAC key on every boot, every URL handed out before the
//! restart would silently fail — a quiet durability shift forbidden
//! by **B3**. So we force the caller to supply a `PresignKey` (in
//! production, fished out of [`SecretStore`](starter_spi::secrets::SecretStore))
//! and document the rotation semantics in their config rather than
//! hiding them in the engine.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use starter_spi::blob::PresignOp;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

/// The HMAC secret used to sign and verify fs-engine presigned URLs.
///
/// 32 bytes; treated as opaque material. Construct via
/// [`PresignKey::from_bytes`] (production: pull the bytes out of a
/// [`SecretStore`](starter_spi::secrets::SecretStore) and pass them
/// in) or [`PresignKey::ephemeral`] (tests).
///
/// `Debug` is redacted; the key is never serialised.
#[derive(Clone)]
pub struct PresignKey(pub(crate) [u8; 32]);

impl PresignKey {
    /// Wrap exactly 32 bytes as a presign key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Try to wrap an arbitrary byte slice. Returns `None` if the
    /// slice is not 32 bytes long.
    pub fn try_from_slice(bytes: &[u8]) -> Option<Self> {
        let arr: [u8; 32] = bytes.try_into().ok()?;
        Some(Self(arr))
    }

    /// Mint a fresh random key. **Test-only semantics:** the key
    /// dies with the process. Any presigned URL issued under an
    /// ephemeral key stops working when the process exits; do not
    /// reach for this in production, or you re-introduce the
    /// durability shift the mandatory-at-construction rule was
    /// designed to prevent.
    pub fn ephemeral() -> Self {
        let mut k = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut k);
        Self(k)
    }

    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Debug for PresignKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PresignKey(<redacted>)")
    }
}

/// Claim embedded in an fs-engine presigned URL. Public for the
/// same reason as in `starter-blob-memory`: same-crate integration
/// tests inspect it; consumers do not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresignClaim {
    /// HTTP verb the URL is signed for.
    pub op: PresignOp,
    /// Engine-internal locator. For the fs engine this is the
    /// key string — consumers must not depend on the encoding.
    pub locator: String,
    /// UNIX epoch seconds at which the signature stops being
    /// honoured.
    pub expires_at_unix: u64,
}

type HmacSha256 = Hmac<Sha256>;

pub(crate) fn sign(key: &PresignKey, claim: &PresignClaim) -> String {
    let payload = serde_json::to_vec(claim).expect("claim is always JSON-encodable");
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC-SHA256 accepts any key length");
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload);
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    format!("{payload_b64}.{sig_b64}")
}

/// Why a token was rejected — internal diagnostic; the router maps
/// every variant onto HTTP 403.
///
/// Used by the `axum` feature's router; without the feature only
/// the unit tests reach for it.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum VerifyError {
    /// Token did not split into `<payload>.<sig>` halves.
    #[error("token is malformed")]
    Malformed,
    /// Payload or signature was not valid URL-safe base64.
    #[error("token base64 did not decode")]
    BadBase64,
    /// Payload base64 decoded but did not parse as a `PresignClaim`.
    #[error("token payload did not decode as a claim")]
    BadClaim,
    /// Signature did not match the locally-stored HMAC key.
    #[error("token signature did not verify")]
    BadSignature,
    /// `expires_at_unix` is in the past.
    #[error("token has expired")]
    Expired,
}

#[allow(dead_code)] // exercised by the `axum` feature's router + unit tests
pub(crate) fn verify(key: &PresignKey, token: &str) -> Result<PresignClaim, VerifyError> {
    let (payload_b64, sig_b64) = token.split_once('.').ok_or(VerifyError::Malformed)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| VerifyError::BadBase64)?;
    let presented_sig = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| VerifyError::BadBase64)?;

    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC-SHA256 accepts any key length");
    mac.update(&payload);
    let expected = mac.finalize().into_bytes();
    if expected.ct_eq(&presented_sig).unwrap_u8() != 1 {
        return Err(VerifyError::BadSignature);
    }

    let claim: PresignClaim =
        serde_json::from_slice(&payload).map_err(|_| VerifyError::BadClaim)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if claim.expires_at_unix <= now {
        return Err(VerifyError::Expired);
    }
    Ok(claim)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> PresignClaim {
        PresignClaim {
            op: PresignOp::Get,
            locator: "k".into(),
            expires_at_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + 60,
        }
    }

    #[test]
    fn ephemeral_keys_differ() {
        // Two ephemeral keys collide with vanishingly small
        // probability; assertion guards against a future bug
        // where ephemeral returns a zero key.
        let a = PresignKey::ephemeral();
        let b = PresignKey::ephemeral();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn debug_is_redacted() {
        let k = PresignKey::from_bytes([1u8; 32]);
        assert_eq!(format!("{k:?}"), "PresignKey(<redacted>)");
    }

    #[test]
    fn roundtrip() {
        let k = PresignKey::ephemeral();
        let t = sign(&k, &claim());
        let back = verify(&k, &t).unwrap();
        assert_eq!(back.locator, "k");
    }

    #[test]
    fn rejects_wrong_key() {
        let a = PresignKey::from_bytes([1u8; 32]);
        let b = PresignKey::from_bytes([2u8; 32]);
        let t = sign(&a, &claim());
        assert!(matches!(
            verify(&b, &t).unwrap_err(),
            VerifyError::BadSignature
        ));
    }
}
