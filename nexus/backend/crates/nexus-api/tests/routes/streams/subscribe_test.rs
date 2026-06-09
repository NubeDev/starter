//! M0.5 acceptance: a browser-style `EventSource` subscribes to a live stream
//! using only the signed token in the URL — no `Authorization` header — and
//! receives ticking `data:` events. Proves the not-Bearer SSE path end to end.

#![cfg(feature = "testing")]

use std::time::Duration;

use futures::StreamExt;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::LiveRunner;
use nexus_store::datasource::Envelope;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_server::testing::TestApp;
use tokio::io::AsyncReadExt;

fn test_state() -> AppState {
    let unused = sqlx::PgPool::connect_lazy("postgres://unused").expect("lazy pool");
    AppState {
        // No datasource/metadata query in this test; unconnected lazy pools.
        metadata: unused.clone(),
        datasource: unused,
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: std::sync::Arc::new(starter_authz::testing::AllowAll),
    }
}

#[tokio::test]
async fn subscribe_streams_events_without_a_bearer_header() {
    let app = TestApp::spawn(serve::router(test_state())).await;
    let client = reqwest::Client::new();

    // 1. Create the stream (this call would be Bearer-authed in production).
    let created: Value = client
        .post(format!("{}/api/v1/streams", app.base_url))
        .json(&json!({ "datasource_id": uuid::Uuid::new_v4(), "sql": "SELECT 1" }))
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

    // 3. Read until the first `data:` frame arrives (the generate source ticks
    //    every second).
    let body = resp.bytes_stream();
    let mut reader =
        tokio_util::io::StreamReader::new(body.map(|r| r.map_err(std::io::Error::other)));
    let mut buf = vec![0u8; 4096];
    let mut seen = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(8);
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
        seen.contains("\"value\""),
        "event payload carries the shaped row: {seen}"
    );

    // Drop the client side of the SSE connection so the server's graceful
    // shutdown isn't waiting on an open stream.
    drop(reader);
    drop(app);
}

#[tokio::test]
async fn subscribe_without_a_token_is_unauthorized() {
    let app = TestApp::spawn(serve::router(test_state())).await;
    let resp = reqwest::Client::new()
        .get(format!(
            "{}/api/v1/streams/{}",
            app.base_url,
            uuid::Uuid::new_v4()
        ))
        .send()
        .await
        .expect("request");
    // No token query param → 400 (missing required query) or 401; either way not 200.
    assert_ne!(resp.status(), 200, "no token must not stream");

    drop(app);
}
