//! Federated query acceptance (RW-05): a request that names `sources` runs the
//! cross-datasource engine path. Two things are proven end-to-end against a real
//! container:
//!
//! 1. **Cross-source join** — two registered Postgres datasources (here, two
//!    aliases pointing at the same container) are joined in one SQL statement
//!    through `POST /datasources/:id/query`, exercising register → seal →
//!    grant-gate → per-source resolve (decrypt) → DataFusion federation → guarded
//!    collect. The result is shaped exactly like the push-down path.
//! 2. **Tenancy boundary** — a federated request that references a datasource id
//!    owned by *another* tenant fails and leaks nothing: the cross-tenant id reads
//!    as not-found under RLS, so the request errors rather than joining a row the
//!    caller's tenant may not see. The engine is `AllowAll`, so a denial here is
//!    the resolve-time tenancy gate, not a grant the test happened to withhold.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::{Envelope, NewDatasource};
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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        datasource_kinds: Arc::new(nexus_api::datasource_kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        ),
        query_cache: nexus_api::cache::CacheConfig::default().build(),
        quotas: nexus_api::quota::TenantQuotas::new(nexus_api::quota::QuotaConfig::default()),
        rate_limiter: nexus_api::ratelimit::TenantRateLimiter::new(
            nexus_api::ratelimit::RateLimitConfig::default(),
        ),
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

/// Register a postgres datasource over the API, pointing back at the test
/// container, and return its id.
async fn register_self(client: &reqwest::Client, base_url: &str, port: u16, name: &str) -> String {
    let created: Value = client
        .post(format!("{base_url}/api/v1/datasources"))
        .json(&json!({
            "name": name,
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "postgres"
        }))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    created["id"].as_str().expect("id").to_string()
}

#[tokio::test]
#[ignore = "requires docker"]
async fn federated_join_across_two_datasources_runs_end_to_end() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    // Two tables on the container the federated SQL will join across aliases.
    sqlx::query("CREATE TABLE devices (id int primary key, name text)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO devices VALUES (1, 'boiler'), (2, 'chiller')")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("CREATE TABLE readings (device_id int, temp_c int)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO readings VALUES (1, 80), (2, 5), (1, 82)")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let devices_id = register_self(&client, &app.base_url, port, "devices-ds").await;
    let readings_id = register_self(&client, &app.base_url, port, "readings-ds").await;

    // The federated request names both sources; `:id` in the path is one of them.
    // The engine registers each as `ds_<alias>` and resolves the join itself.
    let result: Value = client
        .post(format!("{}/api/v1/datasources/{devices_id}/query", app.base_url))
        .json(&json!({
            "sql": "SELECT d.name, r.temp_c \
                    FROM ds_devices d JOIN ds_readings r ON d.id = r.device_id \
                    ORDER BY d.name, r.temp_c",
            "sources": [
                { "alias": "devices", "datasource": devices_id, "table": "devices" },
                { "alias": "readings", "datasource": readings_id, "table": "readings" }
            ]
        }))
        .send()
        .await
        .expect("federated query")
        .json()
        .await
        .expect("body");

    assert_eq!(result["stats"]["row_count"], 3, "the cross-source join produced three rows");
    let pairs: Vec<(String, i64)> = result["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|r| {
            (
                r["name"].as_str().unwrap().to_string(),
                r["temp_c"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        pairs,
        [
            ("boiler".to_string(), 80),
            ("boiler".to_string(), 82),
            ("chiller".to_string(), 5)
        ],
        "rows arrive shaped + ordered by the federated SQL"
    );

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn cross_tenant_federated_source_is_denied_and_leaks_nothing() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    sqlx::query("CREATE TABLE devices (id int primary key, name text)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO devices VALUES (1, 'boiler')")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let envelope = Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap();
    // A datasource owned by a *different* tenant. The acme caller must never be
    // able to fold this id into a federated join.
    let other = nexus_store::datasource::insert(
        &pool,
        &envelope,
        "other-tenant",
        &NewDatasource {
            name: "secret-ds".into(),
            kind: "postgres".into(),
            host: "127.0.0.1".into(),
            port: i32::from(port),
            database: "postgres".into(),
            db_user: "postgres".into(),
            secret: "postgres".into(),
        },
    )
    .await
    .expect("seed other-tenant datasource");

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let mine = register_self(&client, &app.base_url, port, "mine").await;

    // acme references its own datasource AND the other tenant's id. Under RLS the
    // foreign id reads as not-found, so the whole request must fail — never join
    // and never reveal that the foreign datasource exists.
    let resp = client
        .post(format!("{}/api/v1/datasources/{mine}/query", app.base_url))
        .json(&json!({
            "sql": "SELECT a.name FROM ds_mine a JOIN ds_theirs b ON a.id = b.id",
            "sources": [
                { "alias": "mine", "datasource": mine, "table": "devices" },
                { "alias": "theirs", "datasource": other.id.to_string(), "table": "devices" }
            ]
        }))
        .send()
        .await
        .expect("cross-tenant federated query");

    assert!(
        !resp.status().is_success(),
        "a federated reference to another tenant's datasource must fail, not join"
    );
    let body = resp.text().await.unwrap_or_default();
    assert!(
        !body.contains("secret-ds"),
        "the failure must not leak the foreign datasource's name"
    );

    drop(app);
}
