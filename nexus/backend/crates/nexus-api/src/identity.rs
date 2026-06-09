//! Wire the starter identity crates into routers and an authenticator.
//!
//! nexus-api does not reinvent auth: it mounts `starter-auth-users`
//! (`/auth/*`, tenants/teams) and `starter-authz` (`/v1/authz/*`) unchanged, and
//! verifies each request into a `Principal` carrying `tenant_id` + teams via the
//! `AuthAuthenticator`. This module assembles those pieces from one Postgres pool
//! so `main` stays thin.

use std::sync::Arc;

use axum::Router;
use starter_auth_users::routes::{auth_router, AuthState};
use starter_auth_users::store::{PgSessionStore, PgTenantStore, PgTokenStore, PgUserStore};
use starter_auth_users::AuthAuthenticator;
use starter_authz::routes::AuthzRoutesState;
use starter_authz::store::PostgresPolicyStore;
use starter_authz::{authz_router, DbPolicyEngine, StaticRegistry};
use starter_spi::auth::Authenticator;
use starter_spi::authz::ResourceRegistry;
use starter_store_postgres::Pool;

use crate::state::AppState;

/// The mounted identity surface plus the authenticator that protects the product
/// routes.
pub struct Identity {
    pub auth: Router<AppState>,
    pub authz: Router<AppState>,
    pub authenticator: Arc<dyn Authenticator>,
}

/// Build the identity surface from a metadata pool. The same pool backs the
/// auth/authz tables and the product tables — one database, RLS-isolated.
pub async fn build(pool: Pool) -> Result<Identity, String> {
    let users = Arc::new(PgUserStore::new(pool.clone()));
    let sessions = Arc::new(PgSessionStore::new(pool.clone()));
    let tokens = Arc::new(PgTokenStore::new(pool.clone()));
    let tenants = Arc::new(PgTenantStore::new(pool.clone()));

    // The authenticator binds tenant_id + teams onto the verified Principal, so
    // the tenant_of middleware and RLS have a tenant to scope to.
    let authenticator: Arc<dyn Authenticator> = Arc::new(
        AuthAuthenticator::new(users.clone(), sessions.clone(), tokens.clone())
            .with_tenants(tenants.clone()),
    );

    let auth_state = AuthState::new(users, sessions, tokens).with_tenants(tenants);
    let auth = auth_router::<AppState>(auth_state);

    let registry: Arc<dyn ResourceRegistry> = Arc::new(StaticRegistry::new());
    let policy_store = Arc::new(PostgresPolicyStore::new(pool));
    let engine = Arc::new(
        DbPolicyEngine::new(policy_store, registry.clone(), true)
            .await
            .map_err(|e| format!("policy engine: {e}"))?,
    );
    let authz = authz_router::<AppState>(AuthzRoutesState::new(engine, registry));

    Ok(Identity {
        auth,
        authz,
        authenticator,
    })
}
