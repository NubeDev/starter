//! Internal locator codec.
//!
//! Combinators encode a small JSON struct into the
//! [`BlobRef`](starter_spi::blob::BlobRef)'s `opaque_locator`.
//! JSON-in-string is good enough — the locator is opaque to the
//! consumer (B2), so optimising the wire shape would be theatre.
//! Stability matters: a persisted outer `BlobRef` must decode the
//! same way after a restart. The codec is therefore versioned at
//! the top level (`v: 1`) so a future format bump can land without
//! breaking already-written rows.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use starter_spi::blob::BlobError;

/// Versioned envelope around a combinator's locator payload.
///
/// `v` is the schema version; bump it (and add a migration branch)
/// when the payload shape changes. The `payload` is whatever the
/// individual combinator wants to round-trip.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Envelope<T> {
    pub v: u8,
    pub payload: T,
}

pub(crate) fn encode<T: Serialize>(payload: T) -> String {
    let env = Envelope { v: 1, payload };
    serde_json::to_string(&env).expect("combinator locator must serialise")
}

pub(crate) fn decode<T: DeserializeOwned>(s: &str) -> Result<T, BlobError> {
    let env: Envelope<T> = serde_json::from_str(s).map_err(BlobError::backend)?;
    if env.v != 1 {
        return Err(BlobError::backend(LocatorError::UnknownVersion(env.v)));
    }
    Ok(env.payload)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocatorError {
    #[error("unknown combinator locator version {0}")]
    UnknownVersion(u8),
}
