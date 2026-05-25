//! Auth-route handler context. Holds the three stores plus the
//! cookie-naming configuration. Wrapped in `Arc` so the handlers
//! (which close over it) stay cheap to clone per request.

use std::sync::Arc;

use crate::linked_providers::{LinkedProvidersLookup, NoLinkedProviders};
use crate::principal_extras::{NoPrincipalExtras, PrincipalExtrasLookup};
use crate::role::Role;
use crate::signup::mode::SignupMode;
use crate::signup::rate_limit::{MemoryRateLimiter, SignupRateLimiter};
use crate::store::{SessionStore, TenantStore, TokenStore, UserStore};

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
    /// Principal-extras lookup. Defaults to
    /// [`NoPrincipalExtras`] (returns `Value::Null`); consumers
    /// wiring an identity-attribute source (e.g.
    /// `starter-auth-oauth`'s `OAuthPrincipalExtras`) swap it via
    /// [`Self::with_principal_extras`].
    pub principal_extras: Arc<dyn PrincipalExtrasLookup>,
    /// Signup mode. Defaults to [`SignupMode::Disabled`].
    pub signup: SignupMode,
    /// Rate limiter for signup requests. Defaults to
    /// [`MemoryRateLimiter`].
    pub rate_limit: Arc<dyn SignupRateLimiter>,
    /// Optional tenant store. When wired, `POST /auth/token`
    /// resolves an absent `tenant_id` from
    /// [`TenantStore::memberships_for_user`]. When `None`, the
    /// token route requires the client to pass `tenant_id`
    /// explicitly (`400 missing_tenant_id`). See
    /// [`docs/design/auth/token-issuance.md`](https://example.invalid)
    /// for the resolution rules.
    pub tenants: Option<Arc<dyn TenantStore>>,
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
            principal_extras: Arc::new(NoPrincipalExtras),
            signup: SignupMode::Disabled,
            rate_limit: Arc::new(MemoryRateLimiter::new()),
            tenants: None,
        }
    }

    /// Wire a [`TenantStore`] so `POST /auth/token` can resolve
    /// an absent `tenant_id` from the user's memberships. Without
    /// this builder call, `tenant_id` becomes a required request
    /// field on that route (the route still works — it just
    /// fails closed for callers that omit it).
    pub fn with_tenants(mut self, tenants: Arc<dyn TenantStore>) -> Self {
        self.tenants = Some(tenants);
        self
    }

    /// Override the linked-providers lookup. Builder-style so the
    /// common (OAuth-disabled) case stays a single `AuthState::new`
    /// call.
    pub fn with_linked_providers(mut self, lookup: Arc<dyn LinkedProvidersLookup>) -> Self {
        self.linked_providers = lookup;
        self
    }

    /// Override the principal-extras lookup. The OAuth crate
    /// wires `OAuthPrincipalExtras` here so every authenticated
    /// request carries the `oauth.*` attribute block on
    /// `Principal.extra` (SCOPE.md R8).
    pub fn with_principal_extras(mut self, lookup: Arc<dyn PrincipalExtrasLookup>) -> Self {
        self.principal_extras = lookup;
        self
    }

    /// Enable open signup with the given default role.
    pub fn with_signup_open(mut self, default_role: Role) -> Self {
        self.signup = SignupMode::Open { default_role };
        self
    }

    /// Override the signup rate limiter.
    pub fn with_rate_limiter(mut self, limiter: Arc<dyn SignupRateLimiter>) -> Self {
        self.rate_limit = limiter;
        self
    }
}
