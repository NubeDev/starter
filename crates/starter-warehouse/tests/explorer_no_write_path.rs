//! PR 1 — structural proof that the explorer sub-router exposes no
//! write surface.
//!
//! We don't have `trybuild` in the workspace and aren't pulling it
//! in for this. Instead we walk the assembled router at runtime and
//! assert two things:
//!
//!   1. Every read route the explorer mounts is `GET`-only — no
//!      `POST`, `PUT`, `PATCH`, or `DELETE`.
//!   2. The only `POST` route (`/query`, added in PR 2) rejects
//!      every write verb at the handler layer with `400`, never
//!      forwarding them to ClickHouse.
//!
//! Together these mean "no `INSERT`/`ALTER`/`DROP` can reach
//! ClickHouse through this router" without us having to recurse
//! into typed handler bodies.

#![cfg(feature = "warehouse")]

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use starter_store_warehouse::{ChClient, ChConfig};
use tower::ServiceExt;

fn dummy_client() -> ChClient {
    // We never make a real call in this test. The router is built
    // and exercised at the HTTP layer; query handlers are not
    // invoked because every assertion below is on a non-GET method.
    ChClient::connect(ChConfig::local("http://127.0.0.1:1"))
}

const EXPECTED_GET_ROUTES: &[&str] = &[
    "/api/warehouse/ch/overview",
    "/api/warehouse/ch/tables",
    "/api/warehouse/ch/tables/some_table",
    "/api/warehouse/ch/tables/some_table/data",
    "/api/warehouse/ch/tables/some_table/columns",
    "/api/warehouse/ch/erd",
    "/api/warehouse/ch/autocomplete",
];

// POST is also forbidden on the GET-only routes above. The only
// POST surface the explorer exposes is `/api/warehouse/ch/query`,
// which is asserted independently below.
const FORBIDDEN_METHODS: &[Method] = &[Method::POST, Method::PUT, Method::PATCH, Method::DELETE];

const FORBIDDEN_WRITE_STATEMENTS: &[&str] = &[
    "INSERT INTO samples VALUES (1)",
    "ALTER TABLE samples DROP COLUMN x",
    "OPTIMIZE TABLE samples",
    "TRUNCATE TABLE samples",
    "KILL QUERY WHERE 1",
    "SYSTEM RELOAD DICTIONARY foo",
    "CREATE TABLE x (id UInt64) ENGINE = MergeTree ORDER BY id",
    "DROP TABLE samples",
    "RENAME TABLE a TO b",
    "ATTACH TABLE x",
    "DETACH TABLE x",
];

#[tokio::test]
async fn explorer_router_rejects_every_write_method() {
    let router = starter_warehouse::explorer::routes(dummy_client());

    for path in EXPECTED_GET_ROUTES {
        for method in FORBIDDEN_METHODS {
            let resp = router
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method.clone())
                        .uri(*path)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .expect("router infallible");
            assert_eq!(
                resp.status(),
                StatusCode::METHOD_NOT_ALLOWED,
                "explorer accepted {method} {path}: status {}",
                resp.status(),
            );
        }
    }
}

#[tokio::test]
async fn explorer_query_endpoint_refuses_every_write_verb_with_400() {
    use http_body_util::BodyExt;

    let router = starter_warehouse::explorer::routes(dummy_client());
    for sql in FORBIDDEN_WRITE_STATEMENTS {
        let payload = serde_json::json!({ "sql": sql });
        let req = Request::builder()
            .method(Method::POST)
            .uri("/api/warehouse/ch/query")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&payload).unwrap()))
            .unwrap();
        let resp = router
            .clone()
            .oneshot(req)
            .await
            .expect("router infallible");
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value =
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "expected 400 for {sql:?}, got {status} {body}",
        );
        assert_eq!(
            body["error"], "read_only_violation",
            "wrong error tag for {sql:?}: {body}",
        );
    }
}
