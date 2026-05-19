//! Bridge cookie + bearer credentials into a single `Authenticator`.
//!
//! Dispatch rule: the credential string is routed by prefix.
//! - `sak_…` → API token path (`token::verify`).
//! - `sas_…` → session cookie path (`session::verify_session`).
//! - anything else → `Unauthenticated` without a DB hit.

use std::sync::Arc;

use async_trait::async_trait;
use starter_spi::{
    auth::{Authenticator, Principal},
    error::Result,
    Error,
};

use crate::session::{verify_session, SessionError};
use crate::store::{SessionStore, TokenStore, UserStore};
use crate::token::{verify as verify_token, TokenError, TOKEN_PREFIX};

/// Default `Authenticator` impl. Recognises both `Bearer …` API
/// tokens and cookie-derived session ids via the credential prefix.
pub struct AuthAuthenticator {
    users: Arc<dyn UserStore>,
    sessions: Arc<dyn SessionStore>,
    tokens: Arc<dyn TokenStore>,
}

impl AuthAuthenticator {
    /// Build the authenticator from the three concrete stores.
    pub fn new(
        users: Arc<dyn UserStore>,
        sessions: Arc<dyn SessionStore>,
        tokens: Arc<dyn TokenStore>,
    ) -> Self {
        Self {
            users,
            sessions,
            tokens,
        }
    }
}

const SESSION_PREFIX: &str = "sas_";

#[async_trait]
impl Authenticator for AuthAuthenticator {
    async fn verify(&self, credential: &str) -> Result<Principal> {
        if credential.starts_with(TOKEN_PREFIX) {
            verify_token(self.tokens.as_ref(), self.users.as_ref(), credential)
                .await
                .map_err(map_token_err)
        } else if credential.starts_with(SESSION_PREFIX) {
            verify_session(self.sessions.as_ref(), self.users.as_ref(), credential)
                .await
                .map_err(map_session_err)
        } else {
            Err(Error::Unauthenticated)
        }
    }
}

fn map_token_err(e: TokenError) -> Error {
    match e {
        TokenError::Invalid | TokenError::Revoked => Error::Unauthenticated,
        other => {
            tracing::warn!(target: "starter_auth_users", error = %other, "token verify backend failure");
            Error::Internal {
                source: Box::new(other),
            }
        }
    }
}

fn map_session_err(e: SessionError) -> Error {
    match e {
        SessionError::NotFound | SessionError::CsrfMismatch => Error::Unauthenticated,
        other => {
            tracing::warn!(target: "starter_auth_users", error = %other, "session verify backend failure");
            Error::Internal {
                source: Box::new(other),
            }
        }
    }
}
