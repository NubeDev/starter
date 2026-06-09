//! M1c wiring: the assembled server mounts the identity routers and gates the
//! tenant-scoped product routes behind the principal layer.
//!
//! This proves nexus-api's integration — auth routes are mounted, and an
//! unauthenticated request to a tenant-scoped route is refused rather than
//! served or 404'd. The auth crates' own flow tests cover credential
//! verification itself; here we check that nexus wires them in correctly.

#![cfg(feature = "testing")]

use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::state::AppState;
use nexus_api::{identity, serve};
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use serde_json::json;
use starter_server::testing::TestApp;
use starter_store_postgres::testing::with_database;
use starter_store_postgres::Pool;

async fn assembled_app(admin: &sqlx::PgPool) -> TestApp {
    let pool = Pool::from_sqlx(admin.clone());
    nexus_api::bootstrap::migrate_all(&pool)
        .await
        .expect("migrations");
    let id = identity::build(pool).await.expect("identity");

    let state = AppState {
        metadata: admin.clone(),
        datasource: admin.clone(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        flows: nexus_engine::FlowManager::new().expect("flow manager init"),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: id.engine.clone(),
    };
    let router = serve::assemble(state, id.auth, id.authz, id.authenticator);
    TestApp::spawn(router).await
}

#[tokio::test]
#[ignore = "requires docker"]
async fn me_is_unauthorized_without_a_token() {
    let (admin, _guard) = with_database().await;
    let app = assembled_app(admin.sqlx()).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/api/v1/me", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status(), 401, "no principal ⇒ unauthorized");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn datasource_create_is_refused_without_a_principal() {
    let (admin, _guard) = with_database().await;
    let app = assembled_app(admin.sqlx()).await;

    let resp = reqwest::Client::new()
        .post(format!("{}/api/v1/datasources", app.base_url))
        .json(&json!({
            "name": "x", "kind": "postgres", "host": "h", "port": 5432,
            "database": "d", "user": "u", "password": "p"
        }))
        .send()
        .await
        .expect("request");
    // Gated by the principal/tenant guard — refused, not served, not 404.
    assert_eq!(resp.status(), 401);

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn auth_routes_are_mounted() {
    let (admin, _guard) = with_database().await;
    let app = assembled_app(admin.sqlx()).await;

    // A bad login is a 4xx from the mounted handler — crucially NOT a 404, which
    // would mean the auth router was never mounted.
    let resp = reqwest::Client::new()
        .post(format!("{}/auth/login", app.base_url))
        .json(&json!({ "email": "nobody@example.com", "password": "wrong" }))
        .send()
        .await
        .expect("request");
    assert_ne!(
        resp.status(),
        404,
        "auth_router must be mounted at /auth/login"
    );

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn dashboard_routes_are_mounted_and_gated() {
    let (admin, _guard) = with_database().await;
    let app = assembled_app(admin.sqlx()).await;
    let client = reqwest::Client::new();

    // Listing dashboards without a principal is refused (gated), not 404.
    let list = client
        .get(format!("{}/api/v1/dashboards", app.base_url))
        .send()
        .await
        .expect("request");
    assert_eq!(list.status(), 401);

    // Creating one is likewise gated.
    let create = client
        .post(format!("{}/api/v1/dashboards", app.base_url))
        .json(&json!({ "slug": "plant-1", "name": "Plant 1" }))
        .send()
        .await
        .expect("request");
    assert_eq!(create.status(), 401);

    drop(app);
}
