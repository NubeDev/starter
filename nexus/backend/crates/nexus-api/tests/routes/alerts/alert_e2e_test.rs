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
