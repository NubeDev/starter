//! End-to-end round-trip via the in-memory transport:
//! `initialize` → `tools/list` → `tools/call`. Also asserts that the
//! transport binds the same `principal` and `locale` task-locals as
//! HTTP and stdio, so consumers (rubix-agent's pending mcp_disk_test)
//! can rely on the in-memory path the same way they rely on the wire
//! transports. See `docs/design/starter-changes/README.md` (Phase 2b
//! U2) for the contract.

#![cfg(feature = "testing")]

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_mcp::testing::{pair, pair_with_principal};
use starter_mcp::{current_locale, current_principal, ToolRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::tool::{Tool, ToolDefinition};
use starter_spi::Result as SpiResult;

/// Tool that echoes the call context — current locale tag and current
/// principal subject — back to the caller. Lets one test exercise all
/// three task-local bindings the transport must honour.
struct ContextProbe;

#[async_trait]
impl Tool for ContextProbe {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "context-probe".into(),
            description: "Echo locale + principal seen by the tool.".into(),
            input_schema: json!({ "type": "object" }),
        }
    }
    async fn invoke(&self, _input: Value) -> SpiResult<Value> {
        Ok(json!({
            "locale":    current_locale().map(|t| t.as_str().to_string()),
            "principal": current_principal().map(|p| p.subject),
        }))
    }
}

fn registry() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new().register(ContextProbe))
}

#[tokio::test]
async fn full_round_trip_initialize_list_call() {
    let (mut client, _server) = pair(registry());

    // 1. initialize
    let resp = client
        .request(1, "initialize", Value::Null)
        .await
        .expect("initialize round-trip");
    let init = resp.result.expect("initialize returns a result");
    assert_eq!(init["serverInfo"]["name"], "starter-mcp");
    assert!(init["capabilities"]["tools"].is_object());

    // 2. tools/list
    let resp = client
        .request(2, "tools/list", Value::Null)
        .await
        .expect("tools/list round-trip");
    let tools = resp.result.expect("tools/list returns a result");
    assert_eq!(tools["tools"][0]["name"], "context-probe");

    // 3. tools/call
    let resp = client
        .request(
            3,
            "tools/call",
            json!({ "name": "context-probe", "arguments": {} }),
        )
        .await
        .expect("tools/call round-trip");
    let out = resp.result.expect("tools/call returns a result");
    // No principal bound, no initialize-time locale offered — both null.
    assert!(out["structuredContent"]["locale"].is_null());
    assert!(out["structuredContent"]["principal"].is_null());
}

#[tokio::test]
async fn initialize_meta_accept_language_binds_session_locale() {
    let (mut client, _server) = pair(registry());

    // Negotiate session locale via the MCP `_meta` convention — same
    // shape the stdio transport reads.
    let _ = client
        .request(
            1,
            "initialize",
            json!({ "_meta": { "acceptLanguage": "es-AR" } }),
        )
        .await
        .expect("initialize round-trip");

    let resp = client
        .request(
            2,
            "tools/call",
            json!({ "name": "context-probe", "arguments": {} }),
        )
        .await
        .expect("tools/call round-trip");
    let out = resp.result.expect("tools/call returns a result");
    assert_eq!(out["structuredContent"]["locale"], "es-AR");
}

#[tokio::test]
async fn principal_is_bound_for_dispatch() {
    let principal = Principal {
        subject: "user:alice".into(),
        role: Role::Admin,
        scopes: vec![],
        tenant_id: None,
        teams: Vec::new(),
        tenant_scope: Vec::new(),
        extra: Value::Null,
    };
    let (mut client, _server) = pair_with_principal(registry(), principal);

    let _ = client
        .request(1, "initialize", Value::Null)
        .await
        .expect("initialize round-trip");

    let resp = client
        .request(
            2,
            "tools/call",
            json!({ "name": "context-probe", "arguments": {} }),
        )
        .await
        .expect("tools/call round-trip");
    let out = resp.result.expect("tools/call returns a result");
    assert_eq!(out["structuredContent"]["principal"], "user:alice");
}

#[tokio::test]
async fn unknown_method_returns_method_not_found() {
    let (mut client, _server) = pair(registry());

    let resp = client
        .request(1, "does/not/exist", Value::Null)
        .await
        .expect("round-trip");
    let err = resp.error.expect("unknown method must error");
    assert_eq!(err.code, -32601);
}
