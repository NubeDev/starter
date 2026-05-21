//! Happy-path smokes for `/resolve` (R2 + R4) and `/table` (R6).

mod common;

use axum::http::StatusCode;
use common::{build_app, get_json, post_json, trivial_tree};
use serde_json::json;

#[tokio::test]
async fn resolve_returns_render_and_subscriptions_keys() {
    let app = build_app(trivial_tree());
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({ "page_ref": "page-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body was {body}");
    assert!(body.get("render").is_some(), "no render: {body}");
    assert!(
        body.get("subscriptions").is_some(),
        "no subscriptions: {body}",
    );
    assert_eq!(body["render"]["root"]["type"], "page");
}

#[tokio::test]
async fn resolve_unknown_page_is_diagnostics_404() {
    let app = build_app(trivial_tree());
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({ "page_ref": "does-not-exist" }),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "diagnostics");
    assert_eq!(body["items"][0]["code"], "page_not_found");
}

#[tokio::test]
async fn table_paginates_in_memory_engine() {
    let app = build_app(trivial_tree());
    let (status, body) = get_json(
        app,
        "/api/v1/ui/table?source_id=source-1&page=1&size=2",
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body was {body}");
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
    assert_eq!(body["meta"]["total"], 3);
    assert_eq!(body["meta"]["pages"], 2);
}
