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

use crate::principal_extras::{NoPrincipalExtras, PrincipalExtrasLookup};
use crate::session::{verify_session_with_extras, SessionError};
use crate::store::{SessionStore, TenantStore, TokenStore, UserStore};
use crate::token::{verify as verify_token, TokenError, TOKEN_PREFIX};

/// Default `Authenticator` impl. Recognises both `Bearer …` API
/// tokens and cookie-derived session ids via the credential prefix.
pub struct AuthAuthenticator {
    users: Arc<dyn UserStore>,
    sessions: Arc<dyn SessionStore>,
    tokens: Arc<dyn TokenStore>,
    extras: Arc<dyn PrincipalExtrasLookup>,
    /// Optional Phase 7b — when wired, the authenticator populates
    /// `Principal.teams` from `team_slugs_for_user(tenant_id,
    /// user_id)`. Absent in pre-Phase-7b wiring; the principal then
    /// carries an empty `teams` list and any rule referencing
    /// `principal.teams` simply does not match.
    tenants: Option<Arc<dyn TenantStore>>,
}

impl AuthAuthenticator {
    /// Build the authenticator from the three concrete stores.
    /// `Principal.extra` defaults to `Value::Null` — wire a
    /// [`PrincipalExtrasLookup`] via
    /// [`Self::with_principal_extras`] to stamp identity
    /// attributes (`oauth.*`, etc.) on every authenticated request
    /// for the authz layer.
    pub fn new(
        users: Arc<dyn UserStore>,
        sessions: Arc<dyn SessionStore>,
        tokens: Arc<dyn TokenStore>,
    ) -> Self {
        Self {
            users,
            sessions,
            tokens,
            extras: Arc::new(NoPrincipalExtras),
            tenants: None,
        }
    }

    /// Replace the principal-extras lookup. Builder-style so the
    /// common (no-OAuth) wiring keeps a single
    /// `AuthAuthenticator::new` call.
    pub fn with_principal_extras(mut self, extras: Arc<dyn PrincipalExtrasLookup>) -> Self {
        self.extras = extras;
        self
    }

    /// Phase 7b — wire a [`TenantStore`] so the authenticator
    /// populates `Principal.teams` from
    /// `team_slugs_for_user(principal.tenant_id, principal.subject)`
    /// on every verify. Without this builder call the principal's
    /// `teams` list is always empty (and any rule referencing
    /// `principal.teams contains "…"` simply does not match — the
    /// strictly-additive Phase 7b contract).
    pub fn with_tenants(mut self, tenants: Arc<dyn TenantStore>) -> Self {
        self.tenants = Some(tenants);
        self
    }
}

const SESSION_PREFIX: &str = "sas_";

#[async_trait]
impl Authenticator for AuthAuthenticator {
    async fn verify(&self, credential: &str) -> Result<Principal> {
        let mut principal = if credential.starts_with(TOKEN_PREFIX) {
            verify_token(self.tokens.as_ref(), self.users.as_ref(), credential)
                .await
                .map_err(map_token_err)?
        } else if credential.starts_with(SESSION_PREFIX) {
            verify_session_with_extras(
                self.sessions.as_ref(),
                self.users.as_ref(),
                self.extras.as_ref(),
                credential,
            )
            .await
            .map_err(map_session_err)?
        } else {
            return Err(Error::Unauthenticated);
        };

        // Phase 7b — populate teams. A tenantless principal, the
        // super-admin sentinel "*", or a wiring without a
        // TenantStore all yield an empty team list.
        if let (Some(tenants), Some(tenant_id)) =
            (self.tenants.as_ref(), principal.tenant_id.clone())
        {
            if tenant_id != "*" {
                principal.teams = tenants
                    .team_slugs_for_user(&tenant_id, &principal.subject)
                    .await
                    .map_err(|e| {
                        tracing::warn!(
                            target: "starter_auth_users",
                            error = %e,
                            "team lookup failed during authenticator verify"
                        );
                        Error::Internal {
                            source: Box::new(std::io::Error::other(e.to_string())),
                        }
                    })?;
            }
        }
        Ok(principal)
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
