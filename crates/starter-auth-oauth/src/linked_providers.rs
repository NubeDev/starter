//! `OAuthLinkedProviders` — the
//! [`starter_auth_users::LinkedProvidersLookup`] impl that closes
//! the one-way seam between this crate and `starter-auth-users`.
//!
//! Wired into `AuthState::with_linked_providers(...)` on the
//! consumer side, this turns the `password_not_set` login error
//! from "empty list every time" into "the list of providers this
//! user actually linked." The trait lives in `starter-auth-users`
//! (which does **not** depend on this crate); the impl lives here.

use std::sync::Arc;

use async_trait::async_trait;
use starter_auth_users::{LinkedProvidersError, LinkedProvidersLookup};

use crate::identity_store::IdentityStore;

/// Reads `oauth_identities` to answer "which providers can this
/// user sign in with?" Returns the provider ids in `linked_at`
/// ascending so the user sees the longest-held identity first
/// (the canonical order documented on the trait).
pub struct OAuthLinkedProviders {
    store: Arc<dyn IdentityStore>,
}

impl OAuthLinkedProviders {
    /// Wrap an [`IdentityStore`] so the login handler can use it
    /// as a `LinkedProvidersLookup`.
    pub fn new(store: Arc<dyn IdentityStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl LinkedProvidersLookup for OAuthLinkedProviders {
    async fn linked_providers(&self, user_id: &str) -> Result<Vec<String>, LinkedProvidersError> {
        let rows = self
            .store
            .list_for_user(user_id)
            .await
            .map_err(|e| LinkedProvidersError::Backend(e.to_string()))?;
        // De-duplicate while preserving order: a user can in
        // principle have two GitHub identities (two GitHub accounts
        // linked to the same starter user) but the wire shape is
        // the *provider list*, not the *identity list*. The user
        // only needs to know "click GitHub to sign in"; *which*
        // GitHub account is the provider's problem.
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            if !out.contains(&row.provider) {
                out.push(row.provider);
            }
        }
        Ok(out)
    }
}
