//! Integration tests for [`starter_tool_gmail::GmailSendTool`].
//!
//! Each test stands up a `wiremock` server, points a fresh
//! `GmailSendTool` at it via [`GmailConfig::base_url`], and asserts
//! the tool's behaviour for one of the documented paths (happy path,
//! 401 → Unauthenticated, 5xx → HttpStatus). The mock server
//! replaces the real Gmail API end-to-end — no network is touched.

use std::sync::Arc;

use prometheus::Registry;
use serde_json::json;
use starter_spi::tool::Tool;
use starter_spi::{Error as SpiError, SecretString};
use starter_tool_gmail::{GmailConfig, GmailSendTool};
use wiremock::matchers::{bearer_token, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ACCESS_TOKEN: &str = "ya29.test-token";
const USER_ID: &str = "me";

/// Build a tool pointed at the wiremock server. A fresh
/// [`prometheus::Registry`] per test avoids "metric already
/// registered" collisions when the suite runs in one binary.
fn tool_against(server: &MockServer) -> (Arc<Registry>, GmailSendTool) {
    let registry = Arc::new(Registry::new());
    let cfg = GmailConfig {
        oauth_access_token: SecretString::from(ACCESS_TOKEN.to_string()),
        user_id: USER_ID.to_string(),
        base_url: server.uri(),
    };
    let tool = GmailSendTool::new(cfg, &registry).expect("metrics register");
    (registry, tool)
}

/// Gather the `kind` label counts from the
/// `starter_tool_gmail_send_errors_total` family.
fn error_kind_count(registry: &Registry, want_kind: &str) -> u64 {
    let families = registry.gather();
    let fam = families
        .iter()
        .find(|f| f.name() == "starter_tool_gmail_send_errors_total")
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

fn send_path() -> String {
    format!("/gmail/v1/users/{USER_ID}/messages/send")
}

fn minimal_input() -> serde_json::Value {
    json!({
        "to": [{ "address": "b@example.com" }],
        "subject": "hello",
        "text": "hi there",
    })
}

#[tokio::test]
async fn happy_path_returns_message_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(send_path()))
        .and(bearer_token(ACCESS_TOKEN))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "18f3c1aa9b7c4d12",
            "threadId": "18f3c1aa9b7c4d12",
            "labelIds": ["SENT"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (_reg, tool) = tool_against(&server);
    let out = tool.invoke(minimal_input()).await.expect("happy path");
    assert_eq!(out["message_id"], "18f3c1aa9b7c4d12");
}

#[tokio::test]
async fn unauthorized_maps_to_unauthenticated() {
    // Gmail surfaces a missing/expired token as a 401 with a
    // `{"error": {…}}` body. We expect Unauthenticated so a wrapper
    // layer can trigger a refresh without inspecting the source
    // chain.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(send_path()))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "error": {
                "code": 401,
                "message": "Request had invalid authentication credentials.",
                "status": "UNAUTHENTICATED"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(minimal_input())
        .await
        .expect_err("401 should fail");
    assert!(
        matches!(err, SpiError::Unauthenticated),
        "expected Unauthenticated, got {err:?}",
    );
    assert_eq!(error_kind_count(&registry, "auth"), 1);
}

#[tokio::test]
async fn server_5xx_surfaces_as_http_status() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(send_path()))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream offline"))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(minimal_input())
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
async fn latency_histogram_records_a_sample() {
    // Smoke check that R7's required latency histogram actually
    // observes — a regression here would silently break dashboards.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(send_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "x" })))
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let _ = tool.invoke(minimal_input()).await.expect("happy");

    let families = registry.gather();
    let hist = families
        .iter()
        .find(|f| f.name() == "starter_tool_gmail_send_duration_seconds")
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
    // A direct caller (REST/CLI without an MCP dispatcher) must see
    // a clean `Invalid` for a deserialization failure.
    let server = MockServer::start().await;
    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "to": [{ "address": "x@example.com" }] })) // missing subject
        .await
        .expect_err("missing subject");
    assert!(
        matches!(err, SpiError::Invalid { .. }),
        "expected Invalid, got {err:?}",
    );
    assert_eq!(error_kind_count(&registry, "bad_input"), 1);
}

#[tokio::test]
async fn missing_recipients_surfaces_as_invalid() {
    // The RFC 5322 builder rejects a recipient-less message; the
    // tool maps that to `Invalid` via the `Build` variant.
    let server = MockServer::start().await;
    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "subject": "x", "text": "y" }))
        .await
        .expect_err("missing recipients");
    assert!(
        matches!(err, SpiError::Invalid { .. }),
        "expected Invalid, got {err:?}",
    );
    assert_eq!(error_kind_count(&registry, "message_build"), 1);
}

#[tokio::test]
async fn missing_id_surfaces_as_internal() {
    // 2xx without an `id` is a Gmail-side protocol break — we
    // surface that loudly rather than handing the caller an empty
    // message id.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(send_path()))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(minimal_input())
        .await
        .expect_err("missing id should fail");
    let SpiError::Internal { source } = err else {
        panic!("expected Internal, got {err:?}");
    };
    assert!(
        source.to_string().contains("without an `id`"),
        "unexpected: {source}",
    );
    assert_eq!(error_kind_count(&registry, "missing_id"), 1);
}
