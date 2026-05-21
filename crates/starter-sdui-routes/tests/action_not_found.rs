//! R5 — `POST /api/v1/ui/action` with an unregistered handler
//! returns 404 with a `diagnostics`-shaped body whose stable
//! `code` is `"handler_not_found"`.

mod common;

use axum::http::StatusCode;
use common::{build_app, post_json, trivial_tree};
use serde_json::json;

#[tokio::test]
async fn unknown_handler_is_diagnostics_404() {
    let app = build_app(trivial_tree());
    let (status, body) = post_json(
        app,
        "/api/v1/ui/action",
        json!({
            "handler": "no.such.handler",
            "args": {},
            "context": {},
        }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "diagnostics", "body was {body}");
    let item = &body["items"][0];
    assert_eq!(item["severity"], "error");
    assert_eq!(item["code"], "handler_not_found");
}

#[tokio::test]
async fn known_handler_returns_action_response() {
    let app = build_app(trivial_tree());
    let (status, body) = post_json(
        app,
        "/api/v1/ui/action",
        json!({
            "handler": "noop",
            "args": {},
            "context": {},
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["type"], "toast", "body was {body}");
}
