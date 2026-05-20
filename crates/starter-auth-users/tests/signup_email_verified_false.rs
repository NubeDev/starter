//! R7: Signup user has email_verified = false; admin-created user has
//! email_verified = true (the migration default).

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use prometheus::Registry;
use starter_auth_users::{
    admin::create_admin,
    routes::{auth_router, AuthState},
    signup::rate_limit::NoRateLimit,
    store::{SqliteSessionStore, SqliteTokenStore, SqliteUserStore},
    Role,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{testing::TestApp, ServerBuilder};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

#[derive(Clone)]
struct S;

#[tokio::test]
async fn signup_user_email_verified_false() {
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
    let tokens = Arc::new(SqliteTokenStore::new(pool.clone()));

    let auth_state = AuthState::new(users.clone() as _, sessions as _, tokens as _)
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

    // Signup a user.
    let resp = client
        .post(format!("{}/auth/signup", app.base_url))
        .json(&serde_json::json!({"email": "signup@example.com", "password": "a-strong-pass-1"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Check email_verified via direct DB query.
    let signup_verified: bool = email_verified_for(&pool, "signup@example.com").await;
    assert!(
        !signup_verified,
        "signup user should have email_verified = false"
    );

    // Admin-created user gets email_verified = true (migration default).
    create_admin(
        users.as_ref(),
        "admin@example.com",
        "a-strong-pass-1",
        Role::Admin,
    )
    .await
    .unwrap();
    let admin_verified: bool = email_verified_for(&pool, "admin@example.com").await;
    assert!(
        admin_verified,
        "admin-created user should have email_verified = true"
    );
}

async fn email_verified_for(pool: &Pool, email: &str) -> bool {
    let row: (i32,) =
        sqlx::query_as("SELECT email_verified FROM starter_auth_users_users WHERE email = ?")
            .bind(email)
            .fetch_one(pool.sqlx())
            .await
            .unwrap();
    row.0 != 0
}
