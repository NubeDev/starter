//! Alerting acceptance: a rule created over the API, whose query breaches, fires
//! through a webhook channel on one scheduler pass and resolves on the next when
//! the value recovers — proving evaluator + state machine + notification end to
//! end (the M3 "one alert rule fires through a channel" exit criterion).

#![cfg(feature = "testing")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::alerting::schedule;
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
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    state_with_dev(pool, pool.clone())
}

/// State with an explicit dev `datasource` pool, so a test can prove evaluation
/// reads through a rule's own datasource by passing a dead dev pool.
fn state_with_dev(metadata: &sqlx::PgPool, dev: sqlx::PgPool) -> AppState {
    AppState {
        metadata: metadata.clone(),
        datasource: dev,
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
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        datasource_kinds: Arc::new(nexus_api::datasource_kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(metadata.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            metadata.clone(),
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

/// A webhook sink that counts the POSTs it receives.
async fn webhook_sink() -> (String, Arc<AtomicUsize>) {
    let hits = Arc::new(AtomicUsize::new(0));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hits_in = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            hits_in.fetch_add(1, Ordering::SeqCst);
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                .await;
            let _ = sock.flush().await;
        }
    });
    (format!("http://{addr}/"), hits)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn rule_fires_through_a_webhook_then_resolves() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // A one-row gauge the rule reads; flipping its value drives breach/recover.
    sqlx::query("CREATE TABLE gauge (v double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO gauge VALUES (99)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON gauge TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let (hook_url, hits) = webhook_sink().await;

    let state = test_state(&pool);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // A webhook channel, then a rule that fires when v > 90, wired to it.
    let channel: Value = client
        .post(format!("{}/api/v1/alerts/channels", app.base_url))
        .json(&json!({ "name": "ops", "kind": "webhook", "config": { "url": hook_url } }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    let rule: Value = client
        .post(format!("{}/api/v1/alerts/rules", app.base_url))
        .json(&json!({
            "name": "high-gauge",
            "query": "SELECT v FROM gauge",
            "op": "gt",
            "threshold": 90.0,
            "for_secs": 0,
            "interval_secs": 1,
            "channel_ids": [channel_id]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let rule_id = rule["id"].as_str().unwrap();

    // One scheduler pass: the rule is due, breaches (99 > 90), and fires.
    schedule::run_once(&state).await.expect("pass 1");
    assert_eq!(hits.load(Ordering::SeqCst), 1, "fired once through the webhook");

    // A second pass while still breaching must NOT re-notify (the dedup).
    // The claim advanced next_eval_at by 1s, so wait for it to be due again.
    tokio::time::sleep(Duration::from_millis(1100)).await;
    schedule::run_once(&state).await.expect("pass 2");
    assert_eq!(hits.load(Ordering::SeqCst), 1, "still firing → no repeat notify");

    // Recover the value; the next due pass resolves and notifies once more.
    sqlx::query("UPDATE gauge SET v = 10")
        .execute(admin.sqlx())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    schedule::run_once(&state).await.expect("pass 3");
    assert_eq!(hits.load(Ordering::SeqCst), 2, "resolved → one more notify");

    // The event history records both transitions.
    let events: Value = client
        .get(format!("{}/api/v1/alerts/events", app.base_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = events.as_array().unwrap();
    assert_eq!(arr.len(), 2, "one firing + one resolved");
    let transitions: Vec<&str> = arr.iter().map(|e| e["transition"].as_str().unwrap()).collect();
    assert!(transitions.contains(&"firing"));
    assert!(transitions.contains(&"resolved"));

    let _ = rule_id;
    drop(app);
}

/// A webhook sink that records the JSON body of each POST it receives, so a test
/// can assert the rendered notification message, not just the hit count.
async fn recording_sink() -> (String, Arc<std::sync::Mutex<Vec<Value>>>) {
    let bodies = Arc::new(std::sync::Mutex::new(Vec::<Value>::new()));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let bodies_in = bodies.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = vec![0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]);
            if let Some(idx) = req.find("\r\n\r\n") {
                let body = &req[idx + 4..];
                if let Ok(v) = serde_json::from_str::<Value>(body) {
                    bodies_in.lock().unwrap().push(v);
                }
            }
            let _ = sock
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                .await;
            let _ = sock.flush().await;
        }
    });
    (format!("http://{addr}/"), bodies)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn multi_condition_and_rule_fires_only_when_all_conditions_breach() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // Two gauges: the AND rule fires only when both breach. Start with only one
    // breaching, then push the second over to drive the firing transition.
    sqlx::query("CREATE TABLE g1 (v double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("CREATE TABLE g2 (v double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO g1 VALUES (99)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO g2 VALUES (1)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON g1, g2 TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let (hook_url, bodies) = recording_sink().await;

    let state = test_state(&pool);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let channel: Value = client
        .post(format!("{}/api/v1/alerts/channels", app.base_url))
        .json(&json!({ "name": "ops", "kind": "webhook", "config": { "url": hook_url } }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    // A two-condition AND rule with a custom template. The top-level query/op are
    // ignored when `conditions` is set, but the contract still requires them.
    client
        .post(format!("{}/api/v1/alerts/rules", app.base_url))
        .json(&json!({
            "name": "both-high",
            "query": "SELECT 0",
            "op": "gt",
            "threshold": 0.0,
            "for_secs": 0,
            "interval_secs": 1,
            "combinator": "and",
            "conditions": [
                { "query": "SELECT v FROM g1", "reducer": "last", "op": "gt", "threshold": 90.0 },
                { "query": "SELECT v FROM g2", "reducer": "last", "op": "gt", "threshold": 90.0 }
            ],
            "message_template": "{{rule_name}} -> {{state}}",
            "channel_ids": [channel_id]
        }))
        .send()
        .await
        .unwrap();

    // Pass 1: only g1 breaches → AND is false → no fire.
    schedule::run_once(&state).await.expect("pass 1");
    assert_eq!(bodies.lock().unwrap().len(), 0, "AND not satisfied → silent");

    // Push g2 over the threshold; now both breach → fire.
    sqlx::query("UPDATE g2 SET v = 99")
        .execute(admin.sqlx())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1100)).await;
    schedule::run_once(&state).await.expect("pass 2");

    let captured = bodies.lock().unwrap();
    assert_eq!(captured.len(), 1, "both conditions breach → one notify");
    // The rendered template flows through to the webhook payload.
    let msg = captured[0]["message"].as_str().unwrap();
    assert_eq!(msg, "both-high -> firing");

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn no_data_policy_alerting_fires_when_the_query_returns_no_rows() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // An empty table: the query returns no rows, so the reducer yields no value.
    // With no_data_policy = alerting, that absence must itself fire.
    sqlx::query("CREATE TABLE empties (v double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON empties TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    let (hook_url, hits) = webhook_sink().await;

    let state = test_state(&pool);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let channel: Value = client
        .post(format!("{}/api/v1/alerts/channels", app.base_url))
        .json(&json!({ "name": "ops", "kind": "webhook", "config": { "url": hook_url } }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    client
        .post(format!("{}/api/v1/alerts/rules", app.base_url))
        .json(&json!({
            "name": "missing-data",
            "query": "SELECT v FROM empties",
            "op": "gt",
            "threshold": 0.0,
            "for_secs": 0,
            "interval_secs": 1,
            "no_data_policy": "alerting",
            "channel_ids": [channel_id]
        }))
        .send()
        .await
        .unwrap();

    schedule::run_once(&state).await.expect("pass");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "no rows + no_data_policy=alerting → fires"
    );

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn rule_evaluates_against_its_named_datasource_not_the_dev_pool() {
    use nexus_store::datasource::{self, NewDatasource};

    let (admin, _guard) = with_database().await;
    let port = admin.sqlx().connect_options().as_ref().get_port();
    let pool = runtime_pool(admin.sqlx()).await;

    sqlx::query("CREATE TABLE gauge (v double precision)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO gauge VALUES (99)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON gauge TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();

    // The datasource the rule names — pointing at the container with its creds
    // sealed. The dev pool is dead, so a fire proves the query ran through the
    // datasource, not the shared fallback.
    let ds = datasource::insert(
        &pool,
        &Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        "acme",
        &NewDatasource {
            name: "gauge-db".into(),
            kind: "postgres".into(),
            host: "127.0.0.1".into(),
            port: port as i32,
            database: "postgres".into(),
            db_user: "postgres".into(),
            secret: "postgres".into(),
        },
    )
    .await
    .expect("datasource");

    let (hook_url, hits) = webhook_sink().await;
    let dead = sqlx::PgPool::connect_lazy("postgres://nope:nope@127.0.0.1:1/none").unwrap();
    let state = state_with_dev(&pool, dead);
    let router = serve::router(state.clone()).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    let channel: Value = client
        .post(format!("{}/api/v1/alerts/channels", app.base_url))
        .json(&json!({ "name": "ops", "kind": "webhook", "config": { "url": hook_url } }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let channel_id = channel["id"].as_str().unwrap();

    client
        .post(format!("{}/api/v1/alerts/rules", app.base_url))
        .json(&json!({
            "name": "high-gauge",
            "datasource_id": ds.id,
            "query": "SELECT v FROM gauge",
            "op": "gt",
            "threshold": 90.0,
            "for_secs": 0,
            "interval_secs": 1,
            "channel_ids": [channel_id]
        }))
        .send()
        .await
        .unwrap();

    schedule::run_once(&state).await.expect("pass");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the rule queried its datasource (dev pool is dead) and fired"
    );

    drop(app);
}
