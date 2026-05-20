//! R5: Blocklisted password → 400; minimum-length valid password → 200.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use prometheus::Registry;
use starter_auth_users::{
    routes::{auth_router, AuthState},
    signup::rate_limit::NoRateLimit,
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore},
    Role,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

#[derive(Clone)]
struct S;

async fn app() -> (TestApp, reqwest::Client) {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .run()
        .await
        .unwrap();

    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

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
    (app, client)
}

#[tokio::test]
async fn blocklisted_password_returns_400() {
    let (app, client) = app().await;

    // "password1234" is in the blocklist.
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "a@example.com", "password": "password1234"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST.as_u16());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "weak_password");
}

#[tokio::test]
async fn minimum_length_valid_password_succeeds() {
    let (app, client) = app().await;

    // "aaaaaaaaaaaa" = 12 chars, not in the blocklist → should succeed.
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "b@example.com", "password": "aaaaaaaaaaaa"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
