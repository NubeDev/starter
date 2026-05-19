//! Auth-route handler context. Holds the three stores plus the
//! cookie-naming configuration. Wrapped in `Arc` so the handlers
//! (which close over it) stay cheap to clone per request.

use std::sync::Arc;

use crate::linked_providers::{LinkedProvidersLookup, NoLinkedProviders};
use crate::store::{SessionStore, TokenStore, UserStore};

/// Shared state for the `/auth/*` handlers. Build once at startup and
/// hand to [`super::auth_router`].
#[derive(Clone)]
pub struct AuthState {
    /// User table.
    pub users: Arc<dyn UserStore>,
    /// Session table.
    pub sessions: Arc<dyn SessionStore>,
    /// Token table.
    pub tokens: Arc<dyn TokenStore>,
    /// Linked-providers lookup. Defaults to [`NoLinkedProviders`]
    /// (returns `[]`) for consumers that do not wire
    /// `starter-auth-oauth`.
    pub linked_providers: Arc<dyn LinkedProvidersLookup>,
}

impl AuthState {
    /// Build the state from the three stores. The linked-providers
    /// lookup defaults to [`NoLinkedProviders`]; consumers wiring
    /// `starter-auth-oauth` swap it via [`Self::with_linked_providers`].
    pub fn new(
        users: Arc<dyn UserStore>,
        sessions: Arc<dyn SessionStore>,
        tokens: Arc<dyn TokenStore>,
    ) -> Self {
        Self {
            users,
            sessions,
            tokens,
            linked_providers: Arc::new(NoLinkedProviders),
        }
    }

    /// Override the linked-providers lookup. Builder-style so the
    /// common (OAuth-disabled) case stays a single `AuthState::new`
    /// call.
    pub fn with_linked_providers(mut self, lookup: Arc<dyn LinkedProvidersLookup>) -> Self {
        self.linked_providers = lookup;
        self
    }
}
