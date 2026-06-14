//! End-to-end HTTP tests for `POST /auth/token` (credentials →
//! bearer issuance). The cookie-less counterpart of `/auth/login`,
//! used by the mobile / native-desktop / CLI clients. Design doc:
//! `rubix/docs/design/auth/token-issuance.md`.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use prometheus::Registry;
use starter_auth_users::{
    admin::create_admin,
    routes::{auth_router, AuthState},
    store::{
        MembershipRecord, SqliteSessionStore, SqliteTenantStore, SqliteTokenStore, SqliteUserStore,
        TenantRecord, TenantStore, UserStore,
    },
    AuthAuthenticator, Role,
};
use starter_observability::metrics::StandardMetrics;
use starter_server::{auth::with_principal, testing::TestApp, ServerBuilder};
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

fn make_tenant(slug: &str) -> TenantRecord {
    TenantRecord {
        id: uuid::Uuid::new_v4().to_string(),
        slug: slug.into(),
        display_name: slug.into(),
        audit_allow_sample: None,
        parent_id: None,
    }
}

struct Harness {
    app: TestApp,
    client: reqwest::Client,
}

async fn spawn_with_tenants(tenants: Option<Arc<dyn TenantStore>>, pool: Pool) -> Harness {
    let users = Arc::new(SqliteUserStore::new(pool.clone()));
    let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
    let tokens = Arc::new(SqliteTokenStore::new(pool));

    let authenticator = Arc::new(AuthAuthenticator::new(
        users.clone() as _,
        sessions.clone() as _,
        tokens.clone() as _,
    ));

    let mut auth_state = AuthState::new(users as _, sessions as _, tokens as _);
    if let Some(t) = tenants {
        auth_state = auth_state.with_tenants(t);
    }

    let protected: Router<EmptyState> =
        with_principal(Router::new().route("/whoami", get(whoami)), authenticator);

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).unwrap());
    let router = ServerBuilder::<EmptyState>::new(EmptyState)
        .merge_router(auth_router::<EmptyState>(auth_state))
        .merge_router(protected)
        .with_metrics(registry, metrics)
        .build();
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();
    Harness { app, client }
}

#[tokio::test]
async fn token_happy_path_single_membership_authenticates_protected_route() {
    let pool = fresh_pool().await;

    // Seed: one user, one tenant, one membership.
    let users = SqliteUserStore::new(pool.clone());
    create_admin(&users, "u@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();
    let user = users.find_by_email("u@example.com").await.unwrap().unwrap();
    let tenants = Arc::new(SqliteTenantStore::new(pool.clone()));
    let t = make_tenant("acme");
    tenants.create_tenant(&t).await.unwrap();
    tenants
        .add_member(&MembershipRecord {
            tenant_id: t.id.clone(),
            user_id: user.id.clone(),
            role: "reader".into(),
        })
        .await
        .unwrap();

    let h = spawn_with_tenants(Some(tenants), pool).await;

    // Mint.
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "mint");
    let body: serde_json::Value = resp.json().await.unwrap();
    let token = body["token"].as_str().expect("token").to_string();
    assert!(token.starts_with("sak_"), "token shape");
    assert_eq!(body["token_type"], "Bearer");
    assert!(body["expires_at"].is_string(), "expires_at present");

    // Use it.
    let resp = h
        .client
        .get(format!("{}/whoami", h.app.base_url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let principal: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(principal["subject"], user.id);
    assert_eq!(principal["tenant_id"], t.id);

    h.app.shutdown().await;
}

#[tokio::test]
async fn token_multiple_memberships_requires_explicit_tenant_id() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    create_admin(&users, "u@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();
    let user = users.find_by_email("u@example.com").await.unwrap().unwrap();
    let tenants = Arc::new(SqliteTenantStore::new(pool.clone()));
    let a = make_tenant("acme");
    let b = make_tenant("bravo");
    tenants.create_tenant(&a).await.unwrap();
    tenants.create_tenant(&b).await.unwrap();
    for tid in [&a.id, &b.id] {
        tenants
            .add_member(&MembershipRecord {
                tenant_id: tid.clone(),
                user_id: user.id.clone(),
                role: "reader".into(),
            })
            .await
            .unwrap();
    }

    let h = spawn_with_tenants(Some(tenants), pool).await;

    // No tenant_id → 409 with both memberships listed.
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "tenant_required");
    let returned: Vec<String> = body["memberships"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["tenant_id"].as_str().unwrap().to_string())
        .collect();
    assert!(returned.contains(&a.id));
    assert!(returned.contains(&b.id));

    // Explicit pick → 200.
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
            "tenant_id": &b.id,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    let resp = h
        .client
        .get(format!("{}/whoami", h.app.base_url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    let principal: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(principal["tenant_id"], b.id);

    // Bogus pick → 403.
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
            "tenant_id": "no-such-tenant",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    h.app.shutdown().await;
}

#[tokio::test]
async fn token_bad_password_returns_401() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    create_admin(&users, "u@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();

    let h = spawn_with_tenants(None, pool).await;

    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "WRONG",
            "tenant_id": "any",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    h.app.shutdown().await;
}

#[tokio::test]
async fn token_oauth_only_account_returns_password_not_set() {
    // Parity with `/auth/login`: when `password_hash IS NULL`,
    // both routes emit the same `password_not_set` envelope.
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    users
        .create("uid-oauth", "oauth@example.com", None, Role::Reader)
        .await
        .unwrap();

    let h = spawn_with_tenants(None, pool).await;

    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "oauth@example.com",
            "password": "anything",
            "tenant_id": "t",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "password_not_set");
    assert!(body["providers"].is_array());

    h.app.shutdown().await;
}

#[tokio::test]
async fn token_no_tenants_store_requires_explicit_tenant_id() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    create_admin(&users, "u@example.com", "long-enough-pw", Role::Reader)
        .await
        .unwrap();

    let h = spawn_with_tenants(None, pool).await;

    // No tenant_id, no tenants store wired → 400 missing_tenant_id.
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["error"], "missing_tenant_id");

    // Explicit tenant_id → 200 (store does not validate it when
    // memberships are unavailable).
    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "u@example.com",
            "password": "long-enough-pw",
            "tenant_id": "default",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    h.app.shutdown().await;
}

#[tokio::test]
async fn token_admin_with_no_memberships_falls_back_to_super_admin() {
    let pool = fresh_pool().await;
    let users = SqliteUserStore::new(pool.clone());
    create_admin(&users, "admin@example.com", "long-enough-pw", Role::Admin)
        .await
        .unwrap();
    // Tenants store is wired but the admin has zero memberships.
    let tenants = Arc::new(SqliteTenantStore::new(pool.clone()));

    let h = spawn_with_tenants(Some(tenants), pool).await;

    let resp = h
        .client
        .post(format!("{}/auth/token", h.app.base_url))
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "long-enough-pw",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "admin fallback to super-admin");
    let body: serde_json::Value = resp.json().await.unwrap();
    let token = body["token"].as_str().unwrap().to_string();

    let resp = h
        .client
        .get(format!("{}/whoami", h.app.base_url))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let principal: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(principal["tenant_id"], "*");
    assert_eq!(principal["role"], "admin");

    h.app.shutdown().await;
}
