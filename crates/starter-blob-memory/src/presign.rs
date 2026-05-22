//! Presigned-URL token format for the memory engine.
//!
//! # Wire shape
//!
//! `<base64url(claim_json)>.<base64url(hmac_sha256)>`
//!
//! `claim_json` carries `{ op, locator, expires_at_unix }`. The
//! HMAC is keyed by a per-`MemoryBlobStore` random secret minted at
//! construction. The token format is engine-internal: a consumer
//! never inspects it, the router never accepts anything else.
//!
//! # Why HMAC instead of, say, JWT
//!
//! JWT brings header/algorithm negotiation and a much larger spec
//! surface than this engine wants to defend. The presigned token
//! is a single-engine, single-process artefact — the smallest
//! authenticated envelope (`payload.signature`) is the right cost.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use starter_spi::blob::PresignOp;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

/// Claim embedded in a memory-engine presigned URL.
///
/// Public so integration tests in the same crate can build /
/// inspect tokens deliberately; consumers never need this type —
/// they hand the [`PresignedUrl`](starter_spi::blob::PresignedUrl)
/// straight to the client and the engine's router validates on the
/// way in.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresignClaim {
    /// HTTP verb the URL is signed for.
    pub op: PresignOp,
    /// Engine-internal locator. For the memory engine this is the
    /// `BlobKey` string, but consumers must not depend on that.
    pub locator: String,
    /// UNIX epoch seconds at which the signature stops being
    /// honoured.
    pub expires_at_unix: u64,
}

type HmacSha256 = Hmac<Sha256>;

/// Sign a claim with `key`. Returns the wire-format token.
pub(crate) fn sign(key: &[u8], claim: &PresignClaim) -> String {
    let payload = serde_json::to_vec(claim).expect("claim is always JSON-encodable");
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(&payload);
    let sig = mac.finalize().into_bytes();
    let payload_b64 = URL_SAFE_NO_PAD.encode(&payload);
    let sig_b64 = URL_SAFE_NO_PAD.encode(sig);
    format!("{payload_b64}.{sig_b64}")
}

/// Reasons a token is rejected. Translates to HTTP 403 at the
/// router; the variant carries the *why* for tests, not for the
/// client (the client sees only "Forbidden").
///
/// Used by the `axum` feature's router. Without the feature only
/// the unit tests reach for it, so the type is marked
/// `allow(dead_code)` for non-feature builds.
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

/// Verify a token against `key` and the current wall clock. Returns
/// the claim on success.
#[allow(dead_code)] // exercised by the `axum` feature's router + unit tests
pub(crate) fn verify(key: &[u8], token: &str) -> Result<PresignClaim, VerifyError> {
    let (payload_b64, sig_b64) = token.split_once('.').ok_or(VerifyError::Malformed)?;
    let payload = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| VerifyError::BadBase64)?;
    let presented_sig = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| VerifyError::BadBase64)?;

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
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
    fn roundtrip() {
        let key = [7u8; 32];
        let t = sign(&key, &claim());
        let back = verify(&key, &t).unwrap();
        assert_eq!(back.locator, "k");
    }

    #[test]
    fn rejects_wrong_key() {
        let t = sign(&[1u8; 32], &claim());
        let err = verify(&[2u8; 32], &t).unwrap_err();
        assert!(matches!(err, VerifyError::BadSignature));
    }

    #[test]
    fn rejects_expired() {
        let key = [3u8; 32];
        let mut c = claim();
        c.expires_at_unix = 1; // ancient
        let t = sign(&key, &c);
        assert!(matches!(
            verify(&key, &t).unwrap_err(),
            VerifyError::Expired
        ));
    }

    #[test]
    fn rejects_malformed() {
        assert!(matches!(
            verify(&[0u8; 32], "nope").unwrap_err(),
            VerifyError::Malformed
        ));
    }
}
