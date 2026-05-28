//! Boot-time construction of the [`AuthState`] +
//! [`AuthAuthenticator`] pair the agent's HTTP surface uses.
//!
//! Pulls the three Postgres-backed stores (users / sessions /
//! tokens) out of `starter-auth-users::store::postgres` and wires
//! them into the `starter-auth-users::routes::auth_router` contract.
//! Signup stays disabled — operator accounts come from
//! `rubix-admin bootstrap-user`. See
//! [docs/design/auth/](../../../docs/design/auth/README.md).

use std::sync::Arc;

use starter_auth_users::store::{
    PgSessionStore, PgTenantStore, PgTokenStore, PgUserStore, SessionStore, TenantStore,
    TokenStore, UserStore,
};
use starter_auth_users::{routes::AuthState, AuthAuthenticator};
use starter_spi::auth::{Authenticator, Role, Scope};
use starter_store_postgres::Pool;

/// What [`build_auth`] returns: the `AuthState` for the router and
/// the `Authenticator` the protected routers wrap themselves in via
/// `starter_server::auth::with_principal`.
pub struct AuthSurface {
    /// Shared state the `/auth/*` handlers close over.
    pub state: AuthState,
    /// Bearer + cookie authenticator the protected routers use to
    /// resolve the request's [`starter_spi::auth::Principal`].
    pub authenticator: Arc<dyn Authenticator>,
}

/// Build the auth surface from a live Postgres pool.
pub fn build_auth(pool: Pool) -> AuthSurface {
    let users: Arc<dyn UserStore> = Arc::new(PgUserStore::new(pool.clone()));
    let sessions: Arc<dyn SessionStore> = Arc::new(PgSessionStore::new(pool.clone()));
    let tokens: Arc<dyn TokenStore> = Arc::new(PgTokenStore::new(pool.clone()));
    let tenants: Arc<dyn TenantStore> = Arc::new(PgTenantStore::new(pool));
    // Session-derived principals (interactive browser logins via
    // `starter_session` cookie) ship with an empty scope set from
    // `verify_session`. The admin invoke surface is gated by
    // `with_scope("admin:invoke")`, so without role→scope
    // expansion a freshly-logged-in admin would get 403 on every
    // POST against `/admin/registry/tools/{id}/invoke`. API
    // tokens are untouched — their scopes are whatever the
    // operator minted them with.
    let authenticator: Arc<dyn Authenticator> = Arc::new(
        AuthAuthenticator::new(users.clone(), sessions.clone(), tokens.clone())
            .with_session_scopes(admin_role_scopes),
    );
    // `with_tenants` lets `POST /auth/token` (the credentials →
    // bearer route used by mobile / native-desktop / CLI sign-in)
    // resolve an absent `tenant_id` from the user's memberships.
    // Without this builder call the route would fail closed with
    // `400 missing_tenant_id` whenever the client omitted the
    // field — fine for tests, wrong for prod.
    let state = AuthState::new(users, sessions, tokens).with_tenants(tenants);
    AuthSurface {
        state,
        authenticator,
    }
}

/// Role → scopes mapping for interactive cookie sessions in
/// rubix-agent. Mirrors the gates declared on the admin invoke
/// surface in `main.rs`:
///
/// - [`Role::Admin`] → `admin:read`, `admin:invoke`. Lets a
///   browser-logged-in operator both browse the catalog and fire
///   tools through `POST /admin/registry/tools/{id}/invoke`.
/// - [`Role::Writer`] / [`Role::Reader`] → no admin scopes.
///   Lower roles never reach admin-gated surfaces anyway because
///   `with_role(_, Role::Admin)` rejects them first.
///
/// Does **not** apply to API tokens — those keep their explicit
/// minted scope set so operators can attenuate a token to e.g.
/// `admin:read`-only for read-only automation.
fn admin_role_scopes(role: Role) -> Vec<Scope> {
    match role {
        Role::Admin => vec![Scope::new("admin:read"), Scope::new("admin:invoke")],
        Role::Writer | Role::Reader => Vec::new(),
    }
}
