//! Round-trip: HTTP `tools/call` with `Accept-Language: es-AR` →
//! dispatched tool reads `current_locale()` → `Some("es-AR")`.
//!
//! Covers the Phase 2b U1 contract from
//! `docs/design/starter-changes/README.md`: the MCP HTTP transport
//! must bind the `Accept-Language` header as a task-local before
//! dispatching `tools/call`, so tools can render in the caller's
//! language without forking the `Tool` trait.

#![cfg(feature = "http")]

use std::sync::Arc;

use async_trait::async_trait;
use axum::Router;
use serde_json::{json, Value};
use starter_mcp::{current_locale, mcp_router, McpHttpOptions, ToolRegistry};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result as SpiResult;

/// Tool that mirrors `current_locale()` back as `{ "locale": <tag|null> }`.
struct LocaleProbe;

#[async_trait]
impl Tool for LocaleProbe {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "locale-probe".into(),
            description: "Echoes current_locale() to the caller.".into(),
            input_schema: json!({ "type": "object" }),
        }
    }
    async fn invoke(&self, _input: Value) -> SpiResult<Value> {
        Ok(json!({
            "locale": current_locale().map(|t| t.as_str().to_string()),
        }))
    }
}

fn registry() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new().register(LocaleProbe))
}

async fn spawn_app(router: Router<()>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn tools_call_sees_accept_language_header() {
    let app = mcp_router::<()>(registry(), McpHttpOptions::new());
    let base = spawn_app(app).await;

    let resp: Value = reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("accept-language", "es-AR")
        .body(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call",
                "params":{"name":"locale-probe","arguments":{}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(resp["result"]["structuredContent"]["locale"], "es-AR");
}

#[tokio::test]
async fn tools_call_picks_highest_quality_when_multiple_languages() {
    let app = mcp_router::<()>(registry(), McpHttpOptions::new());
    let base = spawn_app(app).await;

    let resp: Value = reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("accept-language", "en;q=0.5, fr;q=0.9, de")
        .body(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call",
                "params":{"name":"locale-probe","arguments":{}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // `de` has the implicit q=1.0, beating fr (0.9) and en (0.5).
    assert_eq!(resp["result"]["structuredContent"]["locale"], "de");
}

#[tokio::test]
async fn tools_call_without_accept_language_sees_none() {
    let app = mcp_router::<()>(registry(), McpHttpOptions::new());
    let base = spawn_app(app).await;

    let resp: Value = reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .body(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call",
                "params":{"name":"locale-probe","arguments":{}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["result"]["structuredContent"]["locale"].is_null());
}

#[tokio::test]
async fn tools_call_with_invalid_accept_language_sees_none() {
    let app = mcp_router::<()>(registry(), McpHttpOptions::new());
    let base = spawn_app(app).await;

    // `en_US` is not BCP-47 — `parse_accept_language` drops it; the
    // handler binds nothing and the tool sees `None`.
    let resp: Value = reqwest::Client::new()
        .post(format!("{base}/mcp"))
        .header("accept-language", "en_US")
        .body(
            r#"{"jsonrpc":"2.0","id":4,"method":"tools/call",
                "params":{"name":"locale-probe","arguments":{}}}"#,
        )
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert!(resp["result"]["structuredContent"]["locale"].is_null());
}
