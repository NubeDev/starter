//! End-to-end HTTP test: mount `auth_router` + a protected route on
//! a TestApp, then login → call protected → logout → confirm 401.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use prometheus::Registry;
use starter_auth_users::{
    admin::create_admin,
    linked_providers::{LinkedProvidersError, LinkedProvidersLookup},
    routes::{auth_router, AuthState},
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore, UserStore},
    AuthAuthenticator, Role,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{
    auth::{with_principal, with_role},
    testing::TestApp,
    ServerBuilder,
};
use starter_spi::auth::Principal;
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");
static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-auth-oauth/migrations/starter_auth_oauth_sqlite");

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .with_source(MigrationSource {
            name: "starter_auth_oauth",
            migrator: &AUTH_OAUTH_SQLITE_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

#[derive(Clone)]
struct EmptyState;

async fn whoami(req: axum::extract::Request) -> axum::response::Response {
    use axum::response::IntoResponse;
    match req.extensions().get::<Principal>() {
        Some(p) => axum::Json(p.clone()).into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

#[tokio::test]
async fn login_logout_round_trip() {
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    create_admin(
        users.as_ref(),
        "u@example.com",
        "long-enough-pw",
        Role::Admin,
    )
    .await
    .unwrap();

    let auth_state = AuthState::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    );
    let authenticator = Arc::new(AuthAuthenticator::new(
        users as _,
        sessions as _,
        tokens as _,
    ));

    let protected: Router<EmptyState> = with_principal(
        with_role(Router::new().route("/whoami", get(whoami)), Role::Admin),
        authenticator,
    );

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .merge_router(auth_router::<EmptyState>(auth_state))
        .merge_router(protected)
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Unauthenticated -> 401
    let resp = client
        .get(format!("{}/whoami", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Login.
    let login = client
        .post(format!("{}/auth/login", app.base_url))
        .json(&serde_json::json!({"email": "u@example.com", "password": "long-enough-pw"}))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 200);
    let body: serde_json::Value = login.json().await.unwrap();
    let csrf = body["csrf_token"].as_str().unwrap().to_string();

    // Authenticated /whoami.
    let resp = client
        .get(format!("{}/whoami", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let principal: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(principal["role"], "admin");

    // Logout without CSRF -> 403
    let resp = client
        .post(format!("{}/auth/logout", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    // Logout with CSRF header -> 204.
    let resp = client
        .post(format!("{}/auth/logout", app.base_url))
        .header("x-csrf-token", &csrf)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Cookie cleared by server -> subsequent /whoami is 401 again.
    // The Set-Cookie with Max-Age=0 should remove the cookie from the
    // jar; if not, the session row is revoked anyway and verify fails.
    let resp = client
        .get(format!("{}/whoami", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    app.shutdown().await;
}

/// Stand-in for the `OAuthLinkedProviders` impl that lives in
/// `starter-auth-oauth`. Returns a canned list so the login route
/// can be tested in isolation.
struct StubLinkedProviders(Vec<String>);

#[async_trait::async_trait]
impl LinkedProvidersLookup for StubLinkedProviders {
    async fn linked_providers(&self, _user_id: &str) -> Result<Vec<String>, LinkedProvidersError> {
        Ok(self.0.clone())
    }
}

#[tokio::test]
async fn login_with_null_password_hash_returns_password_not_set() {
    // OAuth-only user. The login route must respond 400 with the
    // `password_not_set` envelope and the linked-providers list,
    // never 401 (which would tell the caller "wrong password" and
    // invite a password-set flow that does not apply).
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    users
        .create("uid-1", "oauth@example.com", None, Role::Reader)
        .await
        .unwrap();

    let auth_state = AuthState::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    )
    .with_linked_providers(Arc::new(StubLinkedProviders(vec!["github".into()])));

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .merge_router(auth_router::<EmptyState>(auth_state))
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/auth/login", app.base_url))
        .json(&serde_json::json!({"email": "oauth@example.com", "password": "anything"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "password_not_set");
    assert_eq!(body["providers"], serde_json::json!(["github"]));

    app.shutdown().await;
}

#[tokio::test]
async fn login_with_null_password_hash_and_no_linked_providers_returns_empty_list() {
    // Default `AuthState::new` wires `NoLinkedProviders`. The envelope
    // shape is the same; `providers` is just `[]`.
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    users
        .create("uid-2", "lone@example.com", None, Role::Reader)
        .await
        .unwrap();

    let auth_state = AuthState::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    );

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .merge_router(auth_router::<EmptyState>(auth_state))
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/auth/login", app.base_url))
        .json(&serde_json::json!({"email": "lone@example.com", "password": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "password_not_set");
    assert_eq!(body["providers"], serde_json::json!([]));

    app.shutdown().await;
}

#[tokio::test]
async fn reader_blocked_from_admin_route() {
    let pool = fresh_pool().await;
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    create_admin(
        users.as_ref(),
        "r@example.com",
        "long-enough-pw",
        Role::Reader,
    )
    .await
    .unwrap();

    let auth_state = AuthState::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    );
    let authenticator = Arc::new(AuthAuthenticator::new(
        users as _,
        sessions as _,
        tokens as _,
    ));

    let protected: Router<EmptyState> = with_principal(
        with_role(Router::new().route("/admin", get(whoami)), Role::Admin),
        authenticator,
    );

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .merge_router(auth_router::<EmptyState>(auth_state))
        .merge_router(protected)
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    let _ = client
        .post(format!("{}/auth/login", app.base_url))
        .json(&serde_json::json!({"email": "r@example.com", "password": "long-enough-pw"}))
        .send()
        .await
        .unwrap();

    let resp = client
        .get(format!("{}/admin", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    app.shutdown().await;
}
