//! PR 2 — `POST /query` + `table_data` against a live ClickHouse
//! testcontainer. Mirrors `with_stack.rs`: tests are `#[ignore]` so
//! a default `cargo test -p starter-warehouse` stays fast; run
//! with `--ignored` in CI / locally with Docker.

#![cfg(all(feature = "warehouse", feature = "testing"))]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use starter_authz::testing::AllowAll;
use starter_authz::with_permission;
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::PolicyEngine;
use tower::ServiceExt;

fn test_principal() -> Principal {
    Principal {
        subject: "user:test".into(),
        role: Role::Reader,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

fn gated_router(ch: starter_store_warehouse::ChClient) -> axum::Router {
    with_permission(starter_warehouse::explorer::routes(ch), "warehouse", "read")
}

async fn send(
    router: axum::Router,
    method: Method,
    uri: &str,
    body: Option<serde_json::Value>,
) -> (StatusCode, serde_json::Value) {
    let engine: Arc<dyn PolicyEngine> = Arc::new(AllowAll);
    let mut req = Request::builder().method(method).uri(uri);
    if body.is_some() {
        req = req.header("content-type", "application/json");
    }
    let body = match body {
        Some(v) => Body::from(serde_json::to_vec(&v).unwrap()),
        None => Body::empty(),
    };
    let mut req = req.body(body).unwrap();
    req.extensions_mut().insert(test_principal());
    req.extensions_mut().insert(engine);
    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, parsed)
}

#[tokio::test]
#[ignore = "requires Docker for the ClickHouse testcontainer"]
async fn query_round_trips_a_simple_select() {
    let (ch, _g) = starter_store_warehouse::testing::with_clickhouse().await;
    let router = gated_router(ch);

    let (status, body) = send(
        router,
        Method::POST,
        "/api/warehouse/ch/query",
        Some(serde_json::json!({ "sql": "SELECT 1 AS one" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["columns"], serde_json::json!(["one"]));
    let rows = body["rows"].as_array().expect("rows array");
    assert_eq!(rows.len(), 1);
    // ClickHouse returns numerics as JSON numbers under JSONCompact.
    assert_eq!(rows[0][0].as_i64(), Some(1));
}

#[tokio::test]
#[ignore = "requires Docker for the ClickHouse testcontainer"]
async fn query_rejects_writes_at_the_handler() {
    let (ch, _g) = starter_store_warehouse::testing::with_clickhouse().await;
    let router = gated_router(ch);

    let (status, body) = send(
        router,
        Method::POST,
        "/api/warehouse/ch/query",
        Some(serde_json::json!({ "sql": "DROP TABLE x" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "read_only_violation");
}

#[tokio::test]
#[ignore = "requires Docker for the ClickHouse testcontainer"]
async fn table_data_returns_real_rows_after_pr2() {
    let (ch, _g) = starter_store_warehouse::testing::with_clickhouse().await;

    // Seed a tiny table.
    let conn = ch.inner();
    conn.query("CREATE TABLE pr2_seed (id UInt64, label String) ENGINE = MergeTree ORDER BY id")
        .execute()
        .await
        .expect("create");
    conn.query("INSERT INTO pr2_seed (id, label) VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .execute()
        .await
        .expect("seed");

    let router = gated_router(ch);
    let (status, body) = send(
        router,
        Method::GET,
        "/api/warehouse/ch/tables/pr2_seed/data",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_eq!(body["columns"], serde_json::json!(["id", "label"]));
    let rows = body["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][1].as_str(), Some("a"));
    assert_eq!(rows[2][1].as_str(), Some("c"));
}
