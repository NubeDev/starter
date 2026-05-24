//! `GET /openapi.json` — the document the rubix-agent serves to
//! drive `pnpm --filter @nube/rubix-client-ts run codegen`.
//!
//! Drives the openapi-doc router built by
//! [`rubix_agent::routes::openapi_doc::openapi_router`] through
//! `tower`'s `ServiceExt::oneshot`, asserting that the served
//! document:
//!
//!   1. Parses as JSON (the codegen pipeline reads it via fetch).
//!   2. Declares exactly nine `tags` — one per goal area per
//!      `rubix/docs/design/client-ts/README.md`.
//!   3. Carries the canary paths `/healthz` and
//!      `/api/v1/tools/{tool_id}` so a fresh codegen run produces
//!      a callable `RubixClient.dispatch(...)` against the
//!      verb-dispatcher surface.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use rubix_agent::openapi::rubix_openapi;
use rubix_agent::routes::openapi_doc::openapi_router;

async fn body_json(resp: axum::response::Response) -> Value {
    let bytes = to_bytes(resp.into_body(), 1024 * 1024)
        .await
        .expect("body bytes");
    serde_json::from_slice(&bytes).expect("served document parses as JSON")
}

fn get_openapi() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/openapi.json")
        .body(Body::empty())
        .expect("request builds")
}

#[tokio::test]
async fn served_document_parses_as_json() {
    let app = openapi_router(rubix_openapi());
    let resp = app.oneshot(get_openapi()).await.expect("oneshot");
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_object(), "OpenAPI documents are JSON objects");
    let info = body.get("info").expect("info block present");
    assert_eq!(
        info.get("title").and_then(Value::as_str),
        Some("rubix-agent"),
        "info.title pins the document identity",
    );
}

#[tokio::test]
async fn served_document_declares_one_tag_per_goal() {
    let app = openapi_router(rubix_openapi());
    let resp = app.oneshot(get_openapi()).await.expect("oneshot");
    let body = body_json(resp).await;
    let tags = body
        .get("tags")
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("tags array must be present: {body}"));
    assert_eq!(
        tags.len(),
        9,
        "one tag per goal area (auth, system, user-admin, clickhouse-ruler, flow-programmer, mcp, undo, dashboard-stub, weekly-report-stub); got {tags:?}",
    );
    let names: Vec<&str> = tags
        .iter()
        .filter_map(|t| t.get("name").and_then(Value::as_str))
        .collect();
    for expected in [
        "auth",
        "system",
        "user-admin",
        "clickhouse-ruler",
        "flow-programmer",
        "mcp",
        "undo",
        "dashboard-stub",
        "weekly-report-stub",
    ] {
        assert!(
            names.contains(&expected),
            "tag `{expected}` declared in served document; got {names:?}",
        );
    }
}

#[tokio::test]
async fn served_document_includes_canary_paths() {
    let app = openapi_router(rubix_openapi());
    let resp = app.oneshot(get_openapi()).await.expect("oneshot");
    let body = body_json(resp).await;
    let paths = body
        .get("paths")
        .and_then(Value::as_object)
        .unwrap_or_else(|| panic!("paths object must be present: {body}"));
    assert!(
        paths.contains_key("/healthz"),
        "/healthz canary path present in served document; got keys {:?}",
        paths.keys().collect::<Vec<_>>(),
    );
    assert!(
        paths.contains_key("/api/v1/tools/{tool_id}"),
        "/api/v1/tools/{{tool_id}} canary path present in served document; got keys {:?}",
        paths.keys().collect::<Vec<_>>(),
    );
}
