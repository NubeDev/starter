//! PR 5 — REST exposure of `rubix.system.disk`.
//!
//! Drives the tools router built by
//! [`rubix_agent::routes::tools::router`] through `tower`'s
//! `ServiceExt::oneshot`, asserting:
//!
//! 1. `?render=server` round-trips a localised diagnostic in EN
//!    (Accept-Language: en-US) and ES (Accept-Language: es-AR);
//!    same raw `summary` shape, different rendered string.
//! 2. With no `?render` query, the response carries the raw
//!    `Diagnostic` JSON only (no `rendered_summary` field) — server-
//!    side rendering is OFF by default. REST clients render
//!    client-side; only MCP / CLI render server-side.

use std::sync::Arc;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use rubix_agent::registry::build_tool_registry;
use rubix_agent::routes::tools::{router, ToolsState};

fn app() -> axum::Router {
    let bundle = Arc::new(rubix_spi::i18n::rubix_bundle().expect("rubix bundle parses"));
    let tools = build_tool_registry(90, None, None, None, None, None);
    router(ToolsState::new(tools, bundle))
}

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 64 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("body is JSON")
}

fn post_disk(accept_language: &str, render_server: bool) -> Request<Body> {
    let uri = if render_server {
        "/api/v1/tools/rubix.system.disk?render=server"
    } else {
        "/api/v1/tools/rubix.system.disk"
    };
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .header("accept-language", accept_language)
        .body(Body::from("{}"))
        .expect("request builds")
}

#[tokio::test]
async fn render_server_round_trips_in_en_us() {
    let resp = app()
        .oneshot(post_disk("en-US", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let rendered = body
        .get("rendered_summary")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected `rendered_summary`: {body}"));
    // Every host hits one of these three EN openings (ok/warn/full).
    assert!(
        [
            "Disk usage is normal",
            "Disk is nearly full",
            "Disk is full"
        ]
        .iter()
        .any(|p| rendered.starts_with(p)),
        "EN rendering must use English catalogue; got {rendered:?}",
    );
    // Raw summary still present for clients that want to re-render.
    assert!(
        body.get("summary").is_some(),
        "raw summary preserved: {body}"
    );
}

#[tokio::test]
async fn render_server_round_trips_in_es_ar() {
    let resp = app()
        .oneshot(post_disk("es-AR", true))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let rendered = body
        .get("rendered_summary")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("expected `rendered_summary`: {body}"));
    assert!(
        [
            "El uso del disco es normal",
            "El disco está casi lleno",
            "El disco está lleno"
        ]
        .iter()
        .any(|p| rendered.starts_with(p)),
        "ES rendering must use Spanish catalogue; got {rendered:?}",
    );
}

#[tokio::test]
async fn render_off_by_default_carries_raw_diagnostic() {
    let resp = app()
        .oneshot(post_disk("en-US", false))
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(
        body.get("rendered_summary").is_none(),
        "server-side rendering must be OFF without ?render=server; got {body}",
    );
    let summary = body
        .get("summary")
        .unwrap_or_else(|| panic!("response must carry raw Diagnostic: {body}"));
    let code = summary
        .get("code")
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("Diagnostic.code must be a string: {summary}"));
    assert!(
        code.starts_with("rubix.system.disk."),
        "Diagnostic carries a rubix.system.disk.* key; got {code:?}",
    );
}

#[tokio::test]
async fn unknown_tool_returns_404() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tools/rubix.bogus")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request builds"),
        )
        .await
        .expect("oneshot");
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
