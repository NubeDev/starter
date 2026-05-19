//! Bridge cookie + bearer credentials into a single `Authenticator`.

use async_trait::async_trait;
use starter_spi::{
    auth::{Authenticator, Principal},
    error::Result,
    Error,
};

/// Default `Authenticator` impl. Recognises both `Bearer …` API
/// tokens and `starter_session=…` cookies via the `credential` arg.
///
/// The credential string is the transport-extracted value. For
/// cookie auth the server middleware extracts the cookie value
/// before calling `verify`; for bearer auth, the substring after
/// `Bearer `.
pub struct AuthAuthenticator {
    // Fields land with the impl. Sketched empty to lock the public
    // type name + trait impl.
}

#[async_trait]
impl Authenticator for AuthAuthenticator {
    async fn verify(&self, _credential: &str) -> Result<Principal> {
        // TODO(ap): dispatch on credential prefix:
        //   - looks like a session id → session::lookup
        //   - looks like an API token → token::verify
        // Until then, deny everything so the seam is wired but the
        // crate doesn't accidentally accept traffic.
        Err(Error::Unauthenticated)
    }
}
