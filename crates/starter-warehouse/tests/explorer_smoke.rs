//! PR 1 — explorer smoke test.
//!
//! Three independent assertions, none of which need a live
//! ClickHouse container for the auth-gate halves:
//!
//!   * Without a `Principal` in extensions, `/api/warehouse/ch/*`
//!     responds `401 Unauthorized`.
//!   * With a `Principal` plus a `DenyAll` `PolicyEngine`, the same
//!     routes respond `403 Forbidden`.
//!   * Against a real CH testcontainer (the same one
//!     `with_stack.rs` uses), `GET /tables` returns the table list
//!     — proving the route runs the real `tables()` query, not a
//!     hand-rolled stub.
//!
//! The container assertion is `#[ignore]` for consistency with
//! `tests/with_stack.rs`; the auth-gate assertions are unguarded.

#![cfg(all(feature = "warehouse", feature = "testing"))]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use starter_authz::testing::DenyAll;
use starter_authz::with_permission;
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::PolicyEngine;
use starter_store_clickhouse::{ChClient, ChConfig};
use tower::ServiceExt;

fn gated_router(ch: ChClient) -> axum::Router {
    with_permission(
        starter_warehouse::explorer::routes(ch),
        "warehouse",
        "read",
    )
}

fn dummy_client() -> ChClient {
    // The auth-gate tests reject before any query runs; a
    // never-connected client is fine.
    ChClient::connect(ChConfig::local("http://127.0.0.1:1"))
}

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

#[tokio::test]
async fn explorer_returns_401_without_session() {
    let router = gated_router(dummy_client());
    let resp = router
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/ch/tables")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn explorer_returns_403_when_engine_denies_warehouse_read() {
    let router = gated_router(dummy_client());
    let engine: Arc<dyn PolicyEngine> = Arc::new(DenyAll);
    let principal = test_principal();

    let mut req = Request::builder()
        .uri("/api/warehouse/ch/tables")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut().insert(principal);
    req.extensions_mut().insert(engine);

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
#[ignore = "requires Docker for the ClickHouse testcontainer"]
async fn explorer_tables_returns_non_stub_rows_against_real_clickhouse() {
    let (ch, _guard) = starter_store_clickhouse::testing::with_clickhouse().await;

    // Seed a couple of trivial tables so /tables returns more than
    // the empty default. We don't ship a fixture loader at the
    // explorer layer; raw DDL via the store's `inner()` is the
    // shortest path for a smoke test.
    let conn = ch.inner();
    conn.query("CREATE TABLE smoke_one (id UInt64) ENGINE = MergeTree ORDER BY id")
        .execute()
        .await
        .expect("create smoke_one");
    conn.query("CREATE TABLE smoke_two (id UInt64) ENGINE = MergeTree ORDER BY id")
        .execute()
        .await
        .expect("create smoke_two");
    conn.query("INSERT INTO smoke_one (id) VALUES (1)")
        .execute()
        .await
        .expect("seed smoke_one");

    // Auth-bypass: use AllowAll so we exercise the data path. The
    // gating is asserted in the two tests above.
    let engine: Arc<dyn PolicyEngine> = Arc::new(starter_authz::testing::AllowAll);
    let router = gated_router(ch);

    let mut req = Request::builder()
        .uri("/api/warehouse/ch/tables")
        .body(Body::empty())
        .unwrap();
    req.extensions_mut().insert(test_principal());
    req.extensions_mut().insert(engine);

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let tables = parsed["tables"].as_array().expect("tables array");
    let names: Vec<&str> = tables
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        names.iter().any(|n| *n == "smoke_one"),
        "expected smoke_one in /tables, got {names:?}",
    );
    assert!(
        names.iter().any(|n| *n == "smoke_two"),
        "expected smoke_two in /tables, got {names:?}",
    );
    // smoke_one had one row inserted; assert the count came through
    // non-stubbed.
    let one = tables
        .iter()
        .find(|t| t["name"].as_str() == Some("smoke_one"))
        .expect("smoke_one entry");
    assert_eq!(one["count"].as_i64(), Some(1));
}
