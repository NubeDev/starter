//! Integration tests for [`starter_tool_slack::SlackPostTool`].
//!
//! Each test stands up a `wiremock` server, points a fresh
//! `SlackPostTool` at it via [`SlackConfig::base_url`], and asserts the
//! tool's behaviour for one of the four documented paths (success,
//! 429, 5xx, auth failure). The mock server replaces the real Slack
//! API end-to-end — no network is touched.

use std::sync::Arc;

use prometheus::Registry;
use serde_json::json;
use starter_spi::tool::Tool;
use starter_spi::{Error as SpiError, SecretString};
use starter_tool_slack::{SlackConfig, SlackPostTool};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a tool pointed at the wiremock server. A fresh
/// [`prometheus::Registry`] per test avoids "metric already
/// registered" collisions when the suite runs in one binary.
fn tool_against(server: &MockServer) -> (Arc<Registry>, SlackPostTool) {
    let registry = Arc::new(Registry::new());
    let cfg = SlackConfig {
        bot_token: SecretString::from("xoxb-test".to_string()),
        signing_secret: SecretString::from("dummy".to_string()),
        base_url: format!("{}/api", server.uri()),
    };
    let tool = SlackPostTool::new(cfg, &registry).expect("metrics register");
    (registry, tool)
}

/// Gather the `kind` label counts from the
/// `starter_tool_slack_post_errors_total` family. Used to assert that
/// each failure path bumps exactly the right counter.
fn error_kind_count(registry: &Registry, want_kind: &str) -> u64 {
    let families = registry.gather();
    let fam = families
        .iter()
        .find(|f| f.name() == "starter_tool_slack_post_errors_total")
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
async fn happy_path_returns_channel_and_ts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .and(header("Authorization", "Bearer xoxb-test"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "ok": true, "channel": "C1", "ts": "1700.0042" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (_reg, tool) = tool_against(&server);
    let out = tool
        .invoke(json!({ "channel": "C1", "text": "hello" }))
        .await
        .expect("happy path");
    assert_eq!(out["channel"], "C1");
    assert_eq!(out["ts"], "1700.0042");
}

#[tokio::test]
async fn rate_limit_surfaces_with_retry_after() {
    // SCOPE expects the 429 path to be visible to a retry layer
    // through both the structured error variant and the
    // `kind="rate_limited"` counter bump.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "7")
                .set_body_string(""),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "channel": "C1", "text": "hello" }))
        .await
        .expect_err("429 should fail");

    // The SPI error is `Internal { source: SlackError::RateLimited }`.
    // Walk the source chain to assert the structured shape — the SPI
    // error type is intentionally opaque to keep the boundary stable.
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
        .and(path("/api/chat.postMessage"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "channel": "C1", "text": "hello" }))
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
async fn invalid_auth_maps_to_unauthenticated() {
    // Slack returns 200 with `ok=false, error="invalid_auth"` when the
    // bot token is wrong. The mapping into the SPI's `Unauthenticated`
    // variant is what lets an MCP caller distinguish "wrong token"
    // from "Slack is down" without inspecting the source chain.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "ok": false, "error": "invalid_auth" })),
        )
        .expect(1)
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let err = tool
        .invoke(json!({ "channel": "C1", "text": "hello" }))
        .await
        .expect_err("invalid_auth should fail");

    assert!(
        matches!(err, SpiError::Unauthenticated),
        "expected Unauthenticated, got {err:?}",
    );
    // The counter still bumps under the generic `slack_api` label —
    // metric cardinality stays small, the auth case is still visible
    // via the error variant.
    assert_eq!(error_kind_count(&registry, "slack_api"), 1);
}

#[tokio::test]
async fn latency_histogram_records_a_sample() {
    // Smoke check that R7's required latency histogram actually
    // observes — a regression here would silently break dashboards.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "ok": true, "channel": "C1", "ts": "1.0" })),
        )
        .mount(&server)
        .await;

    let (registry, tool) = tool_against(&server);
    let _ = tool
        .invoke(json!({ "channel": "C1", "text": "hello" }))
        .await
        .expect("happy");

    let families = registry.gather();
    let hist = families
        .iter()
        .find(|f| f.name() == "starter_tool_slack_post_duration_seconds")
        .expect("latency histogram present");
    let sample_count: u64 = hist
        .get_metric()
        .iter()
        .map(|m| m.get_histogram().get_sample_count())
        .sum();
    assert_eq!(sample_count, 1, "expected exactly one observation");
}
