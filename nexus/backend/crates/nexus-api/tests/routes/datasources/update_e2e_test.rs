//! Datasource update acceptance: PUT /datasources/:id changes fields under the
//! edit grant and re-seals a rotated secret, so a query that failed under the old
//! (wrong) secret succeeds once the secret is corrected. The route also evicts
//! the cached pool on update — defense for the case where a working pool was
//! already cached when the connection details change.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        datasource_pools: Default::default(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        flows: FlowManager::new().expect("flow manager init"),
        sessions: nexus_api::agents::SessionRunner::new(std::env::temp_dir().join("nexus-knowledge-test"), nexus_skills::BrevityMode::Off),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
    }
}

fn acme_admin() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Admin,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn update_rotates_the_secret_so_a_corrected_query_connects() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    sqlx::query("CREATE TABLE t (n int)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO t VALUES (7)")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Create the datasource with a deliberately WRONG password, so the first
    // query attempt fails to connect — proving the pool is built from the stored
    // secret, not bypassed.
    let created: Value = client
        .post(format!("{}/api/v1/datasources", app.base_url))
        .json(&json!({
            "name": "before",
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "wrong-password"
        }))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    let id = created["id"].as_str().expect("id");

    // The wrong secret cannot connect — a 5xx, not rows.
    let bad = client
        .post(format!("{}/api/v1/datasources/{id}/query", app.base_url))
        .json(&json!({ "sql": "SELECT n FROM t" }))
        .send()
        .await
        .expect("query");
    assert!(bad.status().is_server_error(), "wrong secret cannot connect");

    // Update: rename and rotate the secret to the correct password.
    let updated: Value = client
        .put(format!("{}/api/v1/datasources/{id}", app.base_url))
        .json(&json!({ "name": "after", "password": "postgres" }))
        .send()
        .await
        .expect("update")
        .json()
        .await
        .expect("body");
    assert_eq!(updated["name"], "after");

    // The query now succeeds — the update evicted the bad pool, so this rebuilds
    // with the corrected secret.
    let ok: Value = client
        .post(format!("{}/api/v1/datasources/{id}/query", app.base_url))
        .json(&json!({ "sql": "SELECT n FROM t" }))
        .send()
        .await
        .expect("query")
        .json()
        .await
        .expect("body");
    assert_eq!(ok["rows"][0]["n"], 7, "after eviction the corrected secret connects");

    drop(app);
}
