//! End-to-end smoke against the example binary. Builds the same
//! router the `serve` subcommand uses, exercises the claim flow over
//! HTTP, and verifies the protected route gates on the bearer token.

use std::sync::Arc;

use prometheus::Registry;
use serde_json::Value;
use starter_observability::metrics::StandardMetrics;
use starter_server::testing::TestApp;
use starter_store_sqlite::{migrate, pool};

#[path = "../src/migrations.rs"]
mod migrations;
#[path = "../src/server.rs"]
mod server;

#[tokio::test]
async fn claim_then_hello_round_trip() {
    let pool = pool::connect("sqlite::memory:").await.expect("connect");
    let mut chain = migrate(&pool);
    for source in migrations::sources() {
        chain = chain.with_source(source);
    }
    chain.run().await.expect("migrate");

    // Seed a pending claim row directly.
    let claim_store = starter_auth_token::store::SqliteClaimStore::new(pool.clone());
    let pending = starter_auth_token::regenerate_claim_pending(&claim_store)
        .await
        .expect("seed pending");

    let registry = Arc::new(Registry::new());
    let metrics = Arc::new(StandardMetrics::register(&registry).expect("metrics"));
    let router = server::build(pool, registry, metrics);
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // 1. /hello without auth → 401.
    let resp = client
        .get(format!("{}/hello", app.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // 2. Claim with the seeded pending token → 200 + owner token.
    let claim: Value = client
        .post(format!("{}/auth/claim", app.base_url))
        .json(&serde_json::json!({ "token": pending.plaintext }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let owner = claim["owner_token"]
        .as_str()
        .expect("owner_token")
        .to_string();
    assert!(!owner.is_empty());

    // 3. /hello with bearer → 200 with the principal's subject.
    let body = client
        .get(format!("{}/hello", app.base_url))
        .bearer_auth(&owner)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(body.starts_with("hello, "), "got: {body:?}");

    // 4. MCP over HTTP: POST /mcp without bearer → 401.
    let resp = client
        .post(format!("{}/mcp", app.base_url))
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // 5. MCP tools/list with bearer → echo tool present.
    let listed: Value = client
        .post(format!("{}/mcp", app.base_url))
        .bearer_auth(&owner)
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let tools = listed["result"]["tools"].as_array().expect("tools array");
    assert!(
        tools.iter().any(|t| t["name"] == "echo"),
        "tools/list missing echo: {tools:?}",
    );

    // 6. MCP tools/call: round-trip the echo tool.
    let called: Value = client
        .post(format!("{}/mcp", app.base_url))
        .bearer_auth(&owner)
        .body(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"echo","arguments":{"k":42}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(called["result"]["structuredContent"]["k"], 42);

    app.shutdown().await;
}
