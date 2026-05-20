//! R9: Default config (signup disabled) → route 404s, OpenAPI does not
//! list it.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use prometheus::Registry;
use starter_auth_users::{
    routes::{auth_router, AuthState},
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore},
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

#[derive(Clone)]
struct S;

#[tokio::test]
async fn signup_disabled_returns_404() {
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

    // Default AuthState has signup == Disabled.
    let auth_state = AuthState::new(users as _, sessions as _, tokens as _);

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<S>::new(S)
        .merge_router(auth_router::<S>(auth_state))
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "new@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND.as_u16());
}

#[test]
fn openapi_does_not_list_signup_when_disabled() {
    let doc =
        starter_auth_users::openapi::openapi(&starter_auth_users::signup::SignupMode::Disabled);
    let json = serde_json::to_string(&doc).unwrap();
    assert!(
        !json.contains("/auth/signup"),
        "OpenAPI should not list /auth/signup when disabled"
    );
}
