//! DoS-limit integration tests per SCOPE.md § R8.
//!
//! One test per `what:` tag. The *enforcement* is what these tests
//! pin — the *limit values* are inherited from Rubix and tracked in
//! the SCOPE.md R8 table.
//!
//! Ported in spirit from Rubix's `crates/dashboard-transport/
//! tests/limits.rs`.

mod common;

use axum::http::StatusCode;
use common::{build_app, post_json, trivial_tree};
use serde_json::json;
use starter_ui_ir::{Component, ComponentTree};

#[tokio::test]
async fn page_state_bytes_returns_413_with_tag() {
    let app = build_app(trivial_tree());
    let big = "x".repeat(128 * 1024);
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({
            "page_ref": "page-1",
            "page_state": { "blob": big },
        }),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["what"], "page_state_bytes", "body was {body}");
}

#[tokio::test]
async fn tree_nodes_returns_413_with_tag() {
    let children: Vec<Component> = (0..2_500)
        .map(|i| Component::Text {
            id: Some(format!("t{i}")),
            content: "x".into(),
            intent: None,
            style: None,
        })
        .collect();
    let tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children,
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    let app = build_app(tree);
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({ "page_ref": "page-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["what"], "tree_nodes", "body was {body}");
}

#[tokio::test]
async fn tree_depth_returns_413_with_tag() {
    // 34 nested rows wrapping a leaf, > MAX_TREE_DEPTH (32).
    let mut node = Component::Text {
        id: Some("leaf".into()),
        content: "x".into(),
        intent: None,
        style: None,
    };
    for i in 0..34 {
        node = Component::Row {
            id: Some(format!("r{i}")),
            children: vec![node],
            gap: None,
            layout: None,
            breakpoints: None,
            align: None,
            justify: None,
            wrap: None,
            style: None,
        };
    }
    let tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![node],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    let app = build_app(tree);
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({ "page_ref": "page-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["what"], "tree_depth", "body was {body}");
}

#[tokio::test]
async fn render_tree_bytes_returns_413_with_tag() {
    // One badge with a ~3 MiB label — well over the 2 MiB cap.
    let label = "x".repeat(3 * 1024 * 1024);
    let tree = ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![Component::Badge {
            id: Some("b".into()),
            label,
            intent: None,
            style: None,
        }],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    });
    let app = build_app(tree);
    let (status, body) = post_json(
        app,
        "/api/v1/ui/resolve",
        json!({ "page_ref": "page-1" }),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["what"], "render_tree_bytes", "body was {body}");
}

#[tokio::test]
async fn table_rows_per_page_returns_413_with_tag() {
    let app = build_app(trivial_tree());
    let (status, body) = common::get_json(
        app,
        "/api/v1/ui/table?source_id=source-1&page=1&size=10000",
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["what"], "table_rows_per_page", "body was {body}");
}

#[tokio::test]
async fn handler_timeout_returns_413_with_tag() {
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use starter_sdui_routes::{
        sdui_router, HandlerRegistry, InMemoryPageProvider, InMemoryQueryEngine, SduiState,
    };
    use starter_ui_bindings::NullGraph;
    use tower::ServiceExt;

    // Register a handler that sleeps past the per-fire timeout.
    // `MAX_HANDLER_TIMEOUT` is 5 s — `tokio::time::pause`/`advance`
    // would be cleaner, but the simpler approach is to override the
    // limits constant via a local handler that ticks the runtime
    // forward manually. We use the test-clock approach:
    let registry = HandlerRegistry::new().with("slow", |_ctx| async move {
        // Sleep beyond MAX_HANDLER_TIMEOUT. Wall-clock 6s is too
        // slow for CI; instead use a paused clock and advance past
        // the cap.
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        Ok(starter_ui_ir::ActionResponse::None)
    });
    let state = SduiState::builder()
        .with_pages(InMemoryPageProvider::new())
        .with_entity_graph(NullGraph)
        .with_handler_registry(registry)
        .with_query_engine(InMemoryQueryEngine::new())
        .build()
        .unwrap();
    let app: axum::Router = sdui_router::<()>(state);

    // Pause the runtime clock so the `tokio::time::timeout` in
    // `dispatch` sees a deterministic deadline. The handler's
    // `sleep` then advances past it instantly.
    tokio::time::pause();
    let body = json!({ "handler": "slow", "args": {}, "context": {} });

    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/ui/action")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();

    // Drive the request and let the test clock auto-advance past
    // the 5 s cap inside `dispatch`.
    let task = tokio::spawn(app.oneshot(req));
    // Advance well past MAX_HANDLER_TIMEOUT.
    tokio::time::advance(std::time::Duration::from_secs(10)).await;
    let resp = task.await.unwrap().unwrap();

    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(json["what"], "handler_timeout", "body was {json}");
}

#[tokio::test]
async fn component_types_what_tag_is_stable() {
    // Confirm the tag string is stable on the wire — pinning the
    // serde rename. (Forcing > 60 distinct variants in one tree
    // requires more vocabulary than the IR currently exposes; the
    // SCOPE R8 row notes this is inherited / unmeasured. The
    // structural check is exercised in unit tests of `limits.rs`
    // and the test below pins the wire tag.)
    use starter_sdui_routes::WhatTag;
    assert_eq!(WhatTag::ComponentTypes.as_str(), "component_types");
}
