//! Phase 7a — `/v1/tenants/*` admin REST surface.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use starter_auth_users::{
    routes::tenants_router,
    store::{SqliteTenantStore, TenantStore},
};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

static AUTH_USERS_MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("./migrations/starter_auth_users");

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_auth_users",
            migrator: &AUTH_USERS_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

fn json_req(method: &str, uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn create_then_list_tenant() {
    let pool = fresh_pool().await;
    let tenants: Arc<dyn TenantStore> = Arc::new(SqliteTenantStore::new(pool));
    let app = tenants_router::<()>(tenants);

    let resp = app
        .clone()
        .oneshot(json_req(
            "POST",
            "/v1/tenants",
            serde_json::json!({"slug": "acme", "display_name": "Acme Corp"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .oneshot(json_req("GET", "/v1/tenants", Value::Null))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["slug"], "acme");
}

#[tokio::test]
async fn create_with_reserved_slug_returns_400() {
    let pool = fresh_pool().await;
    let tenants: Arc<dyn TenantStore> = Arc::new(SqliteTenantStore::new(pool));
    let app = tenants_router::<()>(tenants);

    let resp = app
        .oneshot(json_req(
            "POST",
            "/v1/tenants",
            serde_json::json!({"slug": "admin", "display_name": "x"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["error"], "reserved_slug");
}
