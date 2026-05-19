//! Integration tests for [`starter_tool_telegram::TelegramSendMessageTool`].
//!
//! Each test stands up a `wiremock` server, points a fresh
//! `TelegramSendMessageTool` at it via [`TelegramConfig::base_url`],
//! and asserts the tool's behaviour for one of the documented paths
//! (success, 429, 5xx, auth failure). The mock server replaces the
//! real Bot API end-to-end — no network is touched.

use std::sync::Arc;

use prometheus::Registry;
use serde_json::json;
use starter_spi::tool::Tool;
use starter_spi::{Error as SpiError, SecretString};
use starter_tool_telegram::{TelegramConfig, TelegramSendMessageTool};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "12345:test-secret";

/// Build a tool pointed at the wiremock server. A fresh
/// [`prometheus::Registry`] per test avoids "metric already
/// registered" collisions when the suite runs in one binary.
fn tool_against(server: &MockServer) -> (Arc<Registry>, TelegramSendMessageTool) {
    let registry = Arc::new(Registry::new());
    let cfg = TelegramConfig {
        bot_token: SecretString::from(TOKEN.to_string()),
        base_url: server.uri(),
    };
    let tool = TelegramSendMessageTool::new(cfg, &registry).expect("metrics register");
    (registry, tool)
}

/// Gather the `kind` label counts from the
/// `starter_tool_telegram_send_message_errors_total` family. Used to
/// assert that each failure path bumps exactly the right counter.
fn error_kind_count(registry: &Registry, want_kind: &str) -> u64 {
    let families = registry.gather();
    let fam = families
        .iter()
        .find(|f| f.name() == "starter_tool_telegram_send_message_errors_total")
        .expect("error counter present");
    for m in fam.get_metric() {
        let kind = m
            .get_label()
            .iter()
            .find(|l| l.name() == "kind")
            .map(|l| l.value())
            .unwrap_or("");
        if kind == want_kind {
            return m.get_counter().value() as u64;
        }
    }
    0
}

#[tokio::test]
async fn happy_path_returns_message_id_and_chat_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": { "message_id": 99, "chat": { "id": 555 } }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (_reg, tool) = tool_against(&server);
    let out = tool
        .invoke(json!({ "chat_id": 555, "text": "hello" }))
        .await
        .expect("happy path");
    assert_eq!(out["message_id"], 99);
    assert_eq!(out["chat_id"], 555);
}

#[tokio::test]
async fn rate_limit_surfaces_with_retry_after() {
    // SCOPE expects the 429 path to be visible to a retry layer
    // through both the structured error variant and the
    // `kind="rate_limited"` counter bump. The Bot API surfaces
    // retry_after inside `parameters` on the JSON body.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(429).set_body_json(json!({
            "ok": false,
            "error_code": 429,
            "description": "Too Many Requests: retry after 7",
            "parameters": { "retry_after": 7 }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "chat_id": 555, "text": "hello" }))
        .await
        .expect_err("429 should fail");

    let SpiError::Internal { source } = err else {
        panic!("expected Internal, got {err:?}");
    };
    let msg = source.to_string();
    assert!(
        msg.contains("rate-limited") && msg.contains("retry_after_secs=Some(7)"),
        "unexpected error message: {msg}",
    );
    assert_eq!(error_kind_count(&registry, "rate_limited"), 1);
}

#[tokio::test]
async fn server_5xx_surfaces_as_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "chat_id": 555, "text": "hello" }))
        .await
        .expect_err("5xx should fail");

    let SpiError::Internal { source } = err else {
        panic!("expected Internal, got {err:?}");
    };
    assert!(
        source.to_string().contains("HTTP 503"),
        "unexpected: {source}",
    );
    assert_eq!(error_kind_count(&registry, "http_status"), 1);
}

#[tokio::test]
async fn unauthorized_maps_to_unauthenticated() {
    // The Bot API surfaces a bad token as 200 + `ok=false` +
    // `description: "Unauthorized"`. Map this onto the SPI's
    // `Unauthenticated` variant so an MCP caller can distinguish
    // "wrong token" from "Telegram is down" without inspecting the
    // source chain.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": false,
            "error_code": 401,
            "description": "Unauthorized"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "chat_id": 555, "text": "hello" }))
        .await
        .expect_err("unauthorized should fail");

    assert!(
        matches!(err, SpiError::Unauthenticated),
        "expected Unauthenticated, got {err:?}",
    );
    // The counter still bumps under the generic `bot_api` label —
    // metric cardinality stays small, the auth case is still visible
    // via the error variant.
    assert_eq!(error_kind_count(&registry, "bot_api"), 1);
}

#[tokio::test]
async fn latency_histogram_records_a_sample() {
    // Smoke check that R7's required latency histogram actually
    // observes — a regression here would silently break dashboards.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/sendMessage")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": { "message_id": 1, "chat": { "id": 1 } }
        })))
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let _ = tool
        .invoke(json!({ "chat_id": 1, "text": "hello" }))
        .await
        .expect("happy");

    let families = registry.gather();
    let hist = families
        .iter()
        .find(|f| f.name() == "starter_tool_telegram_send_message_duration_seconds")
        .expect("latency histogram present");
    let sample_count: u64 = hist
        .get_metric()
        .iter()
        .map(|m| m.get_histogram().get_sample_count())
        .sum();
    assert_eq!(sample_count, 1, "expected exactly one observation");
}

#[tokio::test]
async fn bad_input_returns_invalid_not_internal() {
    // The schema-validated input path is the dispatcher's; at the
    // crate boundary a serde failure must still surface cleanly so a
    // direct caller (REST/CLI without an MCP dispatcher) sees the
    // right variant.
    let server = MockServer::start().await;
    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "chat_id": 1 })) // missing required `text`
        .await
        .expect_err("missing text");
    assert!(
        matches!(err, SpiError::Invalid { .. }),
        "expected Invalid, got {err:?}",
    );
    assert_eq!(error_kind_count(&registry, "bad_input"), 1);
}
