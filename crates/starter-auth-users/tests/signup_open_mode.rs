//! R1: Open-mode signup → session cookie → GET /auth/me returns the
//! principal with the same cookie shape as login.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

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

#[tokio::test]
async fn signup_open_mode_happy_path() {
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

    let client = reqwest::Client::builder()
        .cookie_store(true)
        .build()
        .unwrap();

    // Signup.
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "fresh@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["csrf_token"].as_str().is_some());

    // Session cookie is set → GET /auth/me works.
    let me = client
        .get(format!("{}/auth/me", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(me.status(), 200);
    let me_body: serde_json::Value = me.json().await.unwrap();
    assert_eq!(me_body["role"], "reader");
}
