//! Detections & findings acceptance (WS-15): an insight run in detection mode
//! over a meters table produces one finding per offending meter, dedups a meter
//! flagged across consecutive runs into one open finding, auto-resolves a meter
//! that stops offending, and lets the analyst acknowledge a finding through the
//! API — proving runner + dedup + lifecycle end to end.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::detecting::schedule;
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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test-detect"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),

        extensions: nexus_api::extensions::empty_registry(),
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
        canary: Default::default(),
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
async fn detection_emits_per_meter_findings_dedups_and_auto_resolves() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // A per-meter usage table; the detection flags meters over a limit. Mutating
    // a meter's value across runs drives the dedup / auto-resolve transitions.
    sqlx::query("CREATE TABLE usage (meter text, value double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO usage VALUES ('m1', 120), ('m2', 50), ('m3', 200)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON usage TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let state = test_state(&pool);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // The rule is an insight: keep only rows whose value exceeds params.limit.
    // The frame shrinks to the offending meters — the WS "find high usage"
    // pattern — so every returned row is a finding (empty `flag_column`).
    let insight: Value = client
        .post(format!("{}/api/v1/insights", app.base_url))
        .json(&json!({
            "name": "high-usage",
            "script": "df.filter_gt(\"value\", params.limit * 1.0)"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let insight_id = insight["id"].as_str().expect("insight id");

    // The detection: run the insight over `usage`, every returned row a finding
    // (empty flag column), identify findings by meter, carry `value`.
    let detection: Value = client
        .post(format!("{}/api/v1/detections", app.base_url))
        .json(&json!({
            "name": "high-usage-detect",
            "insight_id": insight_id,
            "sql": "SELECT meter, value FROM usage",
            "params": { "limit": 100.0 },
            "flag_column": "",
            "target_columns": ["meter"],
            "value_column": "value",
            "interval_secs": 1
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let detection_id = detection["id"].as_str().expect("detection id");

    // Pass 1: m1 (120) and m3 (200) exceed 100 → two findings; m2 does not.
    schedule::run_once(&state).await.expect("pass 1");
    let open = list_findings(&client, &app.base_url, "open").await;
    assert_eq!(open.len(), 2, "one finding per offending meter");
    let meters: Vec<&str> = open
        .iter()
        .map(|f| f["target"]["meter"].as_str().unwrap())
        .collect();
    assert!(meters.contains(&"m1") && meters.contains(&"m3"));
    let m1 = open.iter().find(|f| f["target"]["meter"] == "m1").unwrap();
    assert_eq!(m1["value"].as_f64(), Some(120.0));
    assert_eq!(m1["context"]["value"].as_f64(), Some(120.0), "context carries the why");

    // Pass 2 (still offending): dedup — the same two meters stay TWO open
    // findings, not four. Wait for the claim's advanced next_eval_at.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    schedule::run_once(&state).await.expect("pass 2");
    let open = list_findings(&client, &app.base_url, "open").await;
    assert_eq!(open.len(), 2, "consecutive flags dedup to one open finding each");

    // m1 recovers; m3 still offends. Pass 3 auto-resolves m1, keeps m3 open.
    sqlx::query("UPDATE usage SET value = 10 WHERE meter = 'm1'")
        .execute(admin.sqlx())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    schedule::run_once(&state).await.expect("pass 3");
    let open = list_findings(&client, &app.base_url, "open").await;
    assert_eq!(open.len(), 1, "m1 cleared → auto-resolved");
    assert_eq!(open[0]["target"]["meter"], "m3");
    let resolved = list_findings(&client, &app.base_url, "resolved").await;
    assert_eq!(resolved.len(), 1, "m1's finding is now resolved");
    assert_eq!(resolved[0]["target"]["meter"], "m1");

    // Acknowledge the open m3 finding: open → acknowledged with acked_by.
    let m3_id = open[0]["id"].as_str().unwrap();
    let ack = client
        .post(format!("{}/api/v1/findings/{}/ack", app.base_url, m3_id))
        .json(&json!({ "note": "investigating" }))
        .send()
        .await
        .unwrap();
    assert_eq!(ack.status(), 204);
    let acked = list_findings(&client, &app.base_url, "acknowledged").await;
    assert_eq!(acked.len(), 1);
    assert_eq!(acked[0]["acked_by"], "alice");
    assert_eq!(acked[0]["note"], "investigating");

    let _ = detection_id;
    drop(app);
}

// A detection whose query joins over a *federated* source (alias → datasource)
// runs through the RW-05 federation engine, not the single-datasource push-down
// path — proving the runner reaches every datasource a panel query can, the gap
// this WS closes. One Postgres source registered as `ds_pg` is the minimal join.
#[tokio::test]
#[ignore = "requires docker"]
async fn federated_detection_emits_findings() {
    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    sqlx::query("CREATE TABLE usage (meter text, value double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO usage VALUES ('m1', 120), ('m2', 50), ('m3', 200)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON usage TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let state = test_state(&pool);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // Register a Postgres datasource pointing back at the container — the
    // detection will reference it as a federated source, not its own datasource.
    let ds: Value = client
        .post(format!("{}/api/v1/datasources", app.base_url))
        .json(&json!({
            "name": "warehouse",
            "kind": "postgres",
            "host": "127.0.0.1",
            "port": port,
            "database": "postgres",
            "user": "postgres",
            "password": "postgres"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ds_id = ds["id"].as_str().expect("datasource id");

    let insight: Value = client
        .post(format!("{}/api/v1/insights", app.base_url))
        .json(&json!({
            "name": "high-usage-fed",
            "script": "df.filter_gt(\"value\", params.limit * 1.0)"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let insight_id = insight["id"].as_str().expect("insight id");

    // The SQL reads `ds_pg` — the federated alias — not a local table; the
    // engine registers the source under that name. `sources` flips the runner to
    // the federation path.
    let detection: Value = client
        .post(format!("{}/api/v1/detections", app.base_url))
        .json(&json!({
            "name": "high-usage-federated",
            "insight_id": insight_id,
            "sql": "SELECT meter, value FROM ds_pg",
            "params": { "limit": 100.0 },
            "sources": [
                { "alias": "pg", "datasource": ds_id, "table": "usage" }
            ],
            "flag_column": "",
            "target_columns": ["meter"],
            "value_column": "value",
            "interval_secs": 300
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let detection_id = detection["id"].as_str().expect("detection id");

    // Run it off-schedule through the federation path, then assert findings.
    let ran = client
        .post(format!("{}/api/v1/detections/{}/run", app.base_url, detection_id))
        .send()
        .await
        .unwrap();
    assert_eq!(ran.status(), 200, "federated run should succeed");

    let open = list_findings(&client, &app.base_url, "open").await;
    assert_eq!(open.len(), 2, "m1 (120) and m3 (200) over 100 → two findings");
    let meters: Vec<&str> = open
        .iter()
        .map(|f| f["target"]["meter"].as_str().unwrap())
        .collect();
    assert!(meters.contains(&"m1") && meters.contains(&"m3"));

    drop(app);
}

async fn list_findings(client: &reqwest::Client, base: &str, status: &str) -> Vec<Value> {
    client
        .get(format!("{base}/api/v1/findings?status={status}"))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}
