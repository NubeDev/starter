//! Compose the demo router.
//!
//! Pipeline (outer → inner):
//!   - `with_principal(authenticator)` resolves the bearer token to
//!     a `Principal` and attaches it to request extensions.
//!   - `Extension(engine)` puts the `PolicyEngine` on the request so
//!     `with_permission(kind, action)` middlewares can find it.
//!   - Each route is wrapped by `with_permission(kind, action)` —
//!     the engine returns Allow/Deny.
//!
//! Two resource kinds:
//!   - `reports` — server-owned, with `read` and `create` actions.
//!   - `weather` — extension-contributed, with `read` and `refresh`
//!     actions. Both endpoints are mounted by the demo because the
//!     manifest's `auth: { require_role }` block only does role
//!     gates; per-user grants need the policy engine.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::{Extension, Router};
use prometheus::Registry;
use starter_auth_users::routes::{auth_router, AuthState};
use starter_auth_users::store::{SessionStore, TokenStore, UserStore};
use starter_auth_users::{
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore},
    AuthAuthenticator,
};
use starter_authz::store::SqlitePolicyStore;
use starter_authz::{DbPolicyEngine, StaticRegistry};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_server::{
    rest_router, router_with_auth, BuiltinRestDispatcher, ExtensionAdmin,
    InMemoryEnablementStore, RestRouterOptions,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::auth::with_principal;
use starter_server::ServerBuilder;
use starter_spi::auth::Authenticator;
use starter_spi::authz::{Ownership, PolicyEngine, ResourceRegistry, ResourceSpec};
use starter_store_sqlite::Pool;

use crate::reports::{self, ReportsState};
use crate::weather;

#[derive(Clone)]
pub struct AppState;

pub struct Built {
    pub router: Router,
}

pub async fn build(
    pool: Pool,
    registry: Arc<Registry>,
    metrics: Arc<StandardMetrics>,
) -> Result<Built> {
    // ---------------------------------------------------------------
    // Auth: cookie sessions + API tokens, sharing one Authenticator.
    // ---------------------------------------------------------------
    let users: Arc<dyn UserStore> = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions: Arc<dyn SessionStore> = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens: Arc<dyn TokenStore> = Arc::new(SqliteTokenStore::new(pool.clone()));
    let authenticator: Arc<dyn Authenticator> = Arc::new(AuthAuthenticator::new(
        users.clone(),
        sessions.clone(),
        tokens.clone(),
    ));

    // ---------------------------------------------------------------
    // AuthZ: register every resource kind the routes can be gated on,
    // then build a DB-backed policy engine over those.
    // ---------------------------------------------------------------
    let res_registry = Arc::new(StaticRegistry::new());
    res_registry.register_spec(ResourceSpec::from_static(
        "reports",
        &["read", "create"],
        Ownership::Subject,
        "Reports",
        "Server-owned report objects.",
    ));
    res_registry.register_spec(ResourceSpec::from_static(
        "weather",
        &["read", "refresh"],
        Ownership::None,
        "Weather",
        "Extension-contributed weather endpoints.",
    ));

    let policy_store = Arc::new(SqlitePolicyStore::new(pool.clone()));
    let engine: Arc<DbPolicyEngine> = Arc::new(
        DbPolicyEngine::new(
            policy_store,
            res_registry.clone() as Arc<dyn ResourceRegistry>,
            /* default_policy = */ true,
        )
        .await
        .context("build policy engine")?,
    );
    let engine_dyn: Arc<dyn PolicyEngine> = engine.clone();

    // ---------------------------------------------------------------
    // Extension load — manifest-driven, **plus the REST adapter
    // mounts the weather routes**. Phase 7d (SCOPE-EXT R15): the
    // manifest's `auth.permission: { resource, action }` block is
    // what wraps each handler in `with_permission(...)`. The host
    // hands `rest_router` the same `ResourceRegistry` the engine
    // uses so an unknown `resource` is a load-time error symmetric
    // with `UnknownRole` (the broken extension refuses to mount;
    // the rest of the host comes up). The previous hand-mounted
    // `weather::router()` has become a docstring on `weather.rs`.
    //
    // Layer order applied by the adapter (innermost-first in the
    // code, outermost-first in the request path):
    //
    //   with_role (outer)
    //     → with_scope
    //       → with_permission (inner, from `auth.permission`)
    //         → handler
    //
    // **Audit consequence:** a role-denied request never reaches
    // the policy engine, so it does not produce a permission-deny
    // row in `starter_authz_decisions`. Dashboards must exclude
    // pre-role rejections from "permission deny rate" panels. Do
    // NOT flip the order to make audit symmetric — the coarse
    // role gate short-circuiting the engine is the intended
    // trade.
    // ---------------------------------------------------------------
    let ext_dir = std::env::var_os("EXTENSIONS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./extensions"));
    let candidates = Loader::scan(&ext_dir).validate_all();
    let mut ext_registry = ExtensionRegistry::new();
    let outcome = Loader::commit(candidates, &mut ext_registry);
    ext_registry.seal();
    let ext_registry = Arc::new(ext_registry);
    tracing::info!(
        validated = outcome.validated,
        failed = outcome.failed,
        dir = %ext_dir.display(),
        "extensions loaded",
    );

    // ---------------------------------------------------------------
    // /auth/* — login, logout, me. Signup is left disabled; users are
    // created by the CLI `user create` subcommand.
    // ---------------------------------------------------------------
    let auth_state = AuthState::new(users.clone(), sessions.clone(), tokens.clone());
    let auth_routes = auth_router::<AppState>(auth_state);

    // ---------------------------------------------------------------
    // /reports — built-in resource. Each route carries its own
    // (kind, action) gate (see `reports::router`).
    // ---------------------------------------------------------------
    let reports = reports::router::<AppState>(ReportsState { pool: pool.clone() });

    // ---------------------------------------------------------------
    // /weather/* — built by `rest_router` from the extension manifest.
    // `BuiltinRestDispatcher` calls into the `BuiltinTable` registered
    // by `weather::builtin_table()`. The adapter wraps each entry in
    // `with_permission(...)` per the manifest's `auth.permission`.
    // ---------------------------------------------------------------
    let weather_builtins = weather::builtin_table();
    let rest_dispatcher = Arc::new(BuiltinRestDispatcher::new(
        weather_builtins,
        ext_registry.clone(),
    ));
    let weather = rest_router::<AppState>(
        ext_registry.clone(),
        rest_dispatcher,
        RestRouterOptions {
            path_prefix: None,
            resource_registry: Some(res_registry.clone() as Arc<dyn ResourceRegistry>),
        },
    )
    .context("build extension REST adapter")?;

    // ---------------------------------------------------------------
    // /extensions/* — admin slice. Lists the loaded extensions; gated
    // to `Role::Admin` by `router_with_auth`. This proves the manifest
    // for `com.acme.weather` was actually loaded and validated.
    // ---------------------------------------------------------------
    let admin = ExtensionAdmin::builder(ext_registry.clone())
        .with_enablement_store(Arc::new(InMemoryEnablementStore::default()))
        .build();
    let admin_router =
        router_with_auth::<AppState, dyn Authenticator>(admin, authenticator.clone());

    // ---------------------------------------------------------------
    // Compose. Apply `Extension(engine)` once at the top so every
    // route sees the same engine on its request extensions; then
    // wrap the whole composed router in `with_principal` so the
    // bearer-token resolution lands before any authz gate runs.
    // ---------------------------------------------------------------
    let protected = Router::<AppState>::new()
        .merge(reports)
        .merge(weather)
        .merge(admin_router)
        .layer(Extension(engine_dyn));

    let protected = with_principal(protected, authenticator.clone());

    let router = ServerBuilder::<AppState>::new(AppState)
        .merge_router(auth_routes)
        .merge_router(protected)
        .with_metrics(registry, metrics)
        .build();

    Ok(Built { router })
}
