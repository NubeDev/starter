//! Live-panel acceptance: a Bearer-authed `POST /streams` authorizes a
//! subscription over a real datasource, then a browser-style `EventSource`
//! connects with only the signed token in the URL — no `Authorization` header —
//! and receives ticking `data:` events carrying rows from the panel's actual
//! SQL. Proves the not-Bearer SSE path and the poll-based live SQL source end to
//! end.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use futures::StreamExt;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::{self, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;
use tokio::io::AsyncReadExt;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
    }
}

fn acme_member() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Writer,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn live_panel_streams_real_sql_rows_without_a_bearer_header() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    // The panel queries this table. It is datasource data (not RLS metadata), so
    // it is created by the admin role; the runtime role the query runs under gets
    // only SELECT — the least-privilege a read-only panel query needs.
    sqlx::query("CREATE TABLE reading (sensor text, value int)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("INSERT INTO reading VALUES ('temp_1', 42)")
        .execute(admin.sqlx())
        .await
        .unwrap();
    sqlx::query("GRANT SELECT ON reading TO nexus_runtime")
        .execute(admin.sqlx())
        .await
        .unwrap();
    let ds = datasource::insert(
        &pool,
        &Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        "acme",
        &NewDatasource {
            name: "local".into(),
            kind: "postgres".into(),
            host: "localhost".into(),
            port: 5432,
            database: "d".into(),
            db_user: "u".into(),
            secret: "p".into(),
        },
    )
    .await
    .expect("datasource");

    let router = serve::router(test_state(&pool)).layer(Extension(acme_member()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // 1. Create the stream (Bearer-authed in production; the injected principal
    //    stands in here). The panel SQL is vetted and parked server-side.
    let created: Value = client
        .post(format!("{}/api/v1/streams", app.base_url))
        .json(&json!({
            "datasource_id": ds.id,
            "sql": "SELECT sensor, value FROM reading ORDER BY sensor"
        }))
        .send()
        .await
        .expect("create request")
        .json()
        .await
        .expect("create body");
    let subscribe_url = created["subscribe_url"].as_str().expect("subscribe_url");

    // 2. Open the SSE stream with ONLY the token in the URL — no auth header.
    let resp = client
        .get(format!("{}{}", app.base_url, subscribe_url))
        .send()
        .await
        .expect("subscribe request");
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("content-type")
        .map(|v| v.to_str().unwrap_or("").contains("text/event-stream"))
        .unwrap_or(false));

    // 3. Read until the first `data:` frame — the poll re-runs the SQL every
    //    couple of seconds, so the row from `reading` arrives shaped by the query.
    let body = resp.bytes_stream();
    let mut reader =
        tokio_util::io::StreamReader::new(body.map(|r| r.map_err(std::io::Error::other)));
    let mut buf = vec![0u8; 4096];
    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let n = tokio::time::timeout_at(deadline, reader.read(&mut buf))
            .await
            .expect("event arrives before deadline")
            .expect("read");
        seen.push_str(&String::from_utf8_lossy(&buf[..n]));
        if seen.contains("data:") {
            break;
        }
    }
    assert!(
        seen.contains("temp_1") && seen.contains("42"),
        "event payload carries the real SQL rows: {seen}"
    );

    drop(reader);
    drop(app);
}

#[tokio::test]
async fn subscribe_without_a_token_is_unauthorized() {
    // No DB needed: a tokenless subscribe is rejected before any stream work.
    let unused = sqlx::PgPool::connect_lazy("postgres://unused").expect("lazy pool");
    let app = TestApp::spawn(serve::router(test_state(&unused))).await;
    let resp = reqwest::Client::new()
        .get(format!(
            "{}/api/v1/streams/{}",
            app.base_url,
            uuid::Uuid::new_v4()
        ))
        .send()
        .await
        .expect("request");
    assert_ne!(resp.status(), 200, "no token must not stream");

    drop(app);
}
