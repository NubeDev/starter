//! Mint and verify the short-lived signed token that authenticates an SSE
//! subscription.
//!
//! A native browser `EventSource` cannot send an `Authorization` header, so the
//! live route cannot use the REST Bearer. Instead `POST /streams` (Bearer-authed)
//! mints a token here, scoped to exactly one stream — its id, datasource, tenant,
//! and required permission — and short-lived. `GET /streams/:id?token=…` verifies
//! it. The token is a standing credential for nothing else: it authorizes one
//! subscription and expires in seconds.
//!
//! Format: `base64url(payload).base64url(hmac_sha256(key, payload))`, where
//! payload is the canonical JSON of [`StreamClaims`]. HMAC, not encryption — the
//! claims are not secret, only their integrity and expiry matter.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// What a stream token asserts. The verifier re-derives the registry key from
/// these, so a token minted for one tenant/datasource cannot subscribe to
/// another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamClaims {
    pub stream_id: String,
    pub datasource_id: String,
    pub tenant_id: String,
    pub permission: String,
    /// Unix-seconds expiry. The verifier rejects a token at or past this.
    pub exp: u64,
}

/// Signs and verifies stream tokens with a process HMAC key.
#[derive(Clone)]
pub struct StreamTokenSigner {
    key: Vec<u8>,
}

/// Why a token failed verification — all map to 401 at the transport, but the
/// distinction aids logging.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("malformed stream token")]
    Malformed,
    #[error("stream token signature mismatch")]
    BadSignature,
    #[error("stream token expired")]
    Expired,
}

impl StreamTokenSigner {
    /// Build a signer from raw key bytes (injected from config/secret store).
    pub fn new(key: impl Into<Vec<u8>>) -> Self {
        Self { key: key.into() }
    }

    /// Mint a token for `claims`.
    pub fn mint(&self, claims: &StreamClaims) -> String {
        let payload = serde_json::to_vec(claims).expect("claims serialize");
        let payload_b64 = URL_SAFE_NO_PAD.encode(&payload);
        let sig = self.sign(payload_b64.as_bytes());
        format!("{payload_b64}.{}", URL_SAFE_NO_PAD.encode(sig))
    }

    /// Verify a token and return its claims. Checks signature first, then
    /// expiry against `now` (unix seconds) — both in constant time for the MAC.
    pub fn verify(&self, token: &str, now: u64) -> Result<StreamClaims, TokenError> {
        let (payload_b64, sig_b64) = token.split_once('.').ok_or(TokenError::Malformed)?;
        let sig = URL_SAFE_NO_PAD
            .decode(sig_b64)
            .map_err(|_| TokenError::Malformed)?;

        let mut mac = HmacSha256::new_from_slice(&self.key).expect("hmac accepts any key length");
        mac.update(payload_b64.as_bytes());
        mac.verify_slice(&sig)
            .map_err(|_| TokenError::BadSignature)?;

        let payload = URL_SAFE_NO_PAD
            .decode(payload_b64)
            .map_err(|_| TokenError::Malformed)?;
        let claims: StreamClaims =
            serde_json::from_slice(&payload).map_err(|_| TokenError::Malformed)?;
        if now >= claims.exp {
            return Err(TokenError::Expired);
        }
        Ok(claims)
    }

    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(&self.key).expect("hmac accepts any key length");
        mac.update(bytes);
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(exp: u64) -> StreamClaims {
        StreamClaims {
            stream_id: "s1".into(),
            datasource_id: "ds1".into(),
            tenant_id: "acme".into(),
            permission: "view".into(),
            exp,
        }
    }

    #[test]
    fn round_trips_a_valid_token() {
        let signer = StreamTokenSigner::new(*b"test-key-0123456789");
        let token = signer.mint(&claims(1000));
        let got = signer.verify(&token, 500).expect("valid");
        assert_eq!(got, claims(1000));
    }

    #[test]
    fn rejects_an_expired_token() {
        let signer = StreamTokenSigner::new(*b"test-key-0123456789");
        let token = signer.mint(&claims(1000));
        assert!(matches!(
            signer.verify(&token, 1000),
            Err(TokenError::Expired)
        ));
    }

    #[test]
    fn rejects_a_tampered_token() {
        let signer = StreamTokenSigner::new(*b"test-key-0123456789");
        let other = StreamTokenSigner::new(*b"different-key-987654");
        let token = signer.mint(&claims(1000));
        // A token signed by a different key must not verify.
        assert!(matches!(
            other.verify(&token, 500),
            Err(TokenError::BadSignature)
        ));
    }

    #[test]
    fn rejects_a_garbage_token() {
        let signer = StreamTokenSigner::new(*b"test-key-0123456789");
        assert!(matches!(
            signer.verify("not-a-token", 0),
            Err(TokenError::Malformed)
        ));
    }
}
