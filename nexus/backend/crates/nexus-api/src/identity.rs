//! Wire the starter identity crates into routers and an authenticator.
//!
//! nexus-api does not reinvent auth: it mounts `starter-auth-users`
//! (`/auth/*`, tenants/teams) and `starter-authz` (`/v1/authz/*`) unchanged, and
//! verifies each request into a `Principal` carrying `tenant_id` + teams via the
//! `AuthAuthenticator`. This module assembles those pieces from one Postgres pool
//! so `main` stays thin.

use std::sync::Arc;

use axum::Router;
use starter_auth_users::routes::{auth_router, tenants_router, AuthState};
use starter_auth_users::store::{PgSessionStore, PgTenantStore, PgTokenStore, PgUserStore};
use starter_auth_users::AuthAuthenticator;
use starter_authz::instances::InstancesRegistry;
use starter_authz::routes::AuthzRoutesState;
use starter_authz::store::{PolicyStore, PostgresPolicyStore};
use starter_authz::{authz_router, DbPolicyEngine, StaticRegistry};
use starter_spi::auth::Authenticator;
use starter_spi::authz::ResourceRegistry;
use starter_store_postgres::Pool;

use crate::authz::{register_nexus_resources, DashboardInstancesProvider, KIND_DASHBOARD};
use crate::state::AppState;

/// The mounted identity surface plus the authenticator that protects the product
/// routes.
pub struct Identity {
    pub auth: Router<AppState>,
    pub authz: Router<AppState>,
    /// Tenant / member / team CRUD (`/v1/tenants/*`). Protected admin routes —
    /// mounted behind the principal layer like `authz`.
    pub tenants: Router<AppState>,
    pub authenticator: Arc<dyn Authenticator>,
    /// The same engine instance the `/v1/authz/*` router writes to. Handlers
    /// hold this to check grants; a grant written through the router reloads
    /// this very engine, so checks see fresh grants without a second handle.
    pub engine: Arc<DbPolicyEngine>,
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

    // The team/member/tenant CRUD router shares the tenant store; build it before
    // moving the store into the auth state.
    let tenants_routes = tenants_router::<AppState>(tenants.clone());

    let auth_state = AuthState::new(users, sessions, tokens).with_tenants(tenants);
    let auth = auth_router::<AppState>(auth_state);

    let registry: Arc<dyn ResourceRegistry> = Arc::new(StaticRegistry::new());
    register_nexus_resources(registry.as_ref());
    let policy_store = Arc::new(PostgresPolicyStore::new(pool.clone()));
    // default_policy = true keeps the built-in role ladder: a tenant admin
    // (role = admin) is allowed every action on every kind, so admins reach
    // their own tenant's resources without an explicit grant. Non-admins match
    // no built-in rule on the nexus action vocabulary (view/edit/manage), so
    // their access comes solely from per-resource grants — which is the sharing
    // model the product wants. The tenant-scoping predicate isolates either way.
    let policy_store_dyn: Arc<dyn PolicyStore> = policy_store.clone();
    let engine = Arc::new(
        DbPolicyEngine::new(policy_store, registry.clone(), true)
            .await
            .map_err(|e| format!("policy engine: {e}"))?,
    );

    // The instances registry powers the authz admin surface's per-dashboard share
    // view: `GET /v1/authz/resources/nexus.dashboard/instances` lists the tenant's
    // dashboards with their effective ACL. Only dashboards opt in for now; other
    // kinds simply 404 on that route until they register a provider.
    let instances = InstancesRegistry::new();
    instances.register(
        KIND_DASHBOARD,
        Arc::new(DashboardInstancesProvider::new(
            pool.sqlx().clone(),
            policy_store_dyn,
        )),
    );
    let authz = authz_router::<AppState>(
        AuthzRoutesState::new(engine.clone(), registry).with_instances(Arc::new(instances)),
    );

    Ok(Identity {
        auth,
        authz,
        tenants: tenants_routes,
        authenticator,
        engine,
    })
}
