//! Shared scaffolding for the route integration tests.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::Value;
use starter_ui_bindings::NullGraph;
use starter_ui_ir::{ActionResponse, Component, ComponentTree, ToastIntent};
use starter_sdui_routes::{
    sdui_router, HandlerRegistry, InMemoryPageProvider, InMemoryQueryEngine, SduiState,
};
use tower::ServiceExt;

pub fn trivial_tree() -> ComponentTree {
    ComponentTree::new(Component::Page {
        id: "p".into(),
        title: None,
        header_actions: vec![],
        children: vec![],
        style: None,
        default_row_gap: None,
        default_column_gap: None,
        default_page_padding: None,
        default_max_width: None,
    })
}

pub fn build_app(tree_for_page: ComponentTree) -> Router {
    let registry = HandlerRegistry::new().with("noop", |_| async move {
        Ok(ActionResponse::Toast {
            intent: ToastIntent::Ok,
            message: "ok".into(),
        })
    });
    let state = SduiState::builder()
        .with_pages(InMemoryPageProvider::new().with("page-1", tree_for_page))
        .with_entity_graph(NullGraph)
        .with_handler_registry(registry)
        .with_query_engine(
            InMemoryQueryEngine::new().with(
                "source-1",
                (0..3)
                    .map(|i| {
                        let mut row = serde_json::Map::new();
                        row.insert("id".into(), Value::from(i));
                        row
                    })
                    .collect(),
            ),
        )
        .build()
        .unwrap();
    sdui_router::<()>(state)
}

pub async fn post_json(app: Router, path: &str, body: Value) -> (StatusCode, Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[allow(dead_code)] // used by `resolve_table.rs` and `limits_413.rs`; the
                    // compiler scopes dead-code per test binary, hence the lint.
pub async fn get_json(app: Router, path: &str) -> (StatusCode, Value) {
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(path)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}
