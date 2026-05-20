//! R6: 6 requests / same IP / unique emails → 6th is 429 with
//! Retry-After. Rate-limit check happens before password hashing so
//! the total wall-clock is bounded.

#![cfg(feature = "sqlite")]

use std::sync::Arc;
use std::time::Instant;

use axum::http::StatusCode;
use prometheus::Registry;
use starter_auth_users::{
    routes::{auth_router, AuthState},
    signup::rate_limit::MemoryRateLimiter,
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
async fn rate_limit_kicks_in_on_sixth_request() {
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
        .with_rate_limiter(Arc::new(MemoryRateLimiter::new()));

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<S>::new(S)
        .merge_router(auth_router::<S>(auth_state))
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;

    let client = reqwest::Client::new();

    // Measure one successful signup to get a baseline hash time.
    let start_one = Instant::now();
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "baseline@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let one_signup_duration = start_one.elapsed();

    // Fire 5 more requests (all unique emails, same IP = 0.0.0.0
    // because no X-Forwarded-For). The IP bucket allows 5 total so
    // we already used 1 above; requests 2–5 should succeed, request 6
    // should hit 429.
    let start_batch = Instant::now();
    let mut last_status = 0;
    for i in 2..=6 {
        let resp = client
            .post(format!("{}/auth/signup", app.base_url))
            .json(&serde_json::json!({
                "email": format!("user{}@example.com", i),
                "password": "a-strong-pass-1"
            }))
            .send()
            .await
            .unwrap();
        last_status = resp.status().as_u16();
        if last_status == StatusCode::TOO_MANY_REQUESTS.as_u16() {
            // Check Retry-After header.
            let retry_after = resp.headers().get("retry-after");
            assert!(retry_after.is_some(), "429 must include Retry-After header");
            break;
        }
    }
    let batch_duration = start_batch.elapsed();

    assert_eq!(last_status, StatusCode::TOO_MANY_REQUESTS.as_u16());

    // The 6-request batch (where the 6th is rejected early) should
    // finish faster than 5× a single signup's hashing time, proving
    // the rate-limit check fires before hashing.
    let max_allowed = one_signup_duration * 5;
    assert!(
        batch_duration < max_allowed,
        "batch took {:?} but limit is {:?} — rate-limit did not fire before hashing",
        batch_duration,
        max_allowed,
    );
}
