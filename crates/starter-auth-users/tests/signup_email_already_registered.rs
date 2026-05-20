//! R4: Collision with a password user and collision with an OAuth-only
//! user return byte-identical 409 bodies.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use prometheus::Registry;
use starter_auth_users::{
    routes::{auth_router, AuthState},
    signup::rate_limit::NoRateLimit,
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore, UserStore},
    Role,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");
static AUTH_OAUTH_SQLITE_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../starter-auth-oauth/migrations/starter_auth_oauth_sqlite");

#[derive(Clone)]
struct S;

#[tokio::test]
async fn email_already_registered_uniform_409() {
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
        .unwrap();

    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    // Pre-create a password user.
    users
        .create(
            "uid-pw",
            "pw@example.com",
            Some("$argon2id$fake"),
            Role::Reader,
        )
        .await
        .unwrap();

    // Pre-create an OAuth-only user (no password).
    users
        .create("uid-oauth", "oauth@example.com", None, Role::Reader)
        .await
        .unwrap();

    let auth_state = AuthState::new(users as _, sessions as _, tokens as _)
        .with_signup_open(Role::Reader)
        .with_rate_limiter(Arc::new(NoRateLimit));

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<S>::new(S)
        .merge_router(auth_router::<S>(auth_state))
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;

    let client = reqwest::Client::new();

    // Try signup with existing password user's email.
    let resp_pw = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "pw@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_pw.status(), StatusCode::CONFLICT.as_u16());
    let body_pw = resp_pw.text().await.unwrap();

    // Try signup with existing OAuth user's email.
    let resp_oauth = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "oauth@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp_oauth.status(), StatusCode::CONFLICT.as_u16());
    let body_oauth = resp_oauth.text().await.unwrap();

    // R4: Bodies must be byte-identical — no field leaks the account type.
    assert_eq!(body_pw, body_oauth);
    assert!(body_pw.contains("email_already_registered"));
}
