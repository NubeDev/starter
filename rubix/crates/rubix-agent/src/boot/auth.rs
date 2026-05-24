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
    PgSessionStore, PgTokenStore, PgUserStore, SessionStore, TokenStore, UserStore,
};
use starter_auth_users::{routes::AuthState, AuthAuthenticator};
use starter_spi::auth::Authenticator;
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
    let tokens: Arc<dyn TokenStore> = Arc::new(PgTokenStore::new(pool));
    let authenticator: Arc<dyn Authenticator> = Arc::new(AuthAuthenticator::new(
        users.clone(),
        sessions.clone(),
        tokens.clone(),
    ));
    let state = AuthState::new(users, sessions, tokens);
    AuthSurface {
        state,
        authenticator,
    }
}
