//! End-to-end tests against a real TimescaleDB testcontainer.
//!
//! Seeds:
//!   * a regular table (`widgets`) with a primary key
//!   * a child table (`widget_events`) with a foreign key into
//!     `widgets` — exercises the `/erd` relationship path
//!   * a hypertable (`metrics`) — exercises the hypertable code
//!     paths in `/overview` and `/tables`
//!
//! These tests are gated on docker being available; they are
//! flagged `#[ignore]` so `cargo test -p starter-warehouse-explorer`
//! stays fast in a no-docker dev loop. Run with
//! `cargo test -p starter-warehouse-explorer -- --ignored`.
//!
//! The contract under test is the **wire shape** — the same JSON
//! the frontend reviver consumes. The handlers are exercised
//! through `tower::ServiceExt::oneshot` rather than a live
//! listener, mirroring the rest of the starter test suite.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::Executor;
use starter_store_warehouse::testing::with_timescale;
use starter_warehouse_explorer::router;
use tower::ServiceExt;

async fn seed(pool: &sqlx::PgPool) {
    pool.execute(
        r#"
        CREATE TABLE widgets (
            id   bigserial PRIMARY KEY,
            name text NOT NULL,
            kind text
        );
        CREATE INDEX widgets_name_idx ON widgets(name);
        INSERT INTO widgets(name, kind) VALUES ('alpha', 'a'), ('beta', 'b'), ('gamma', NULL);

        CREATE TABLE widget_events (
            id        bigserial PRIMARY KEY,
            widget_id bigint NOT NULL REFERENCES widgets(id),
            ts        timestamptz NOT NULL DEFAULT now(),
            payload   jsonb
        );
        INSERT INTO widget_events(widget_id, payload)
            VALUES (1, '{"k":"v"}'::jsonb), (2, NULL);

        CREATE TABLE metrics (
            ts     timestamptz NOT NULL,
            tenant text        NOT NULL,
            value  double precision
        );
        SELECT create_hypertable('metrics', 'ts', chunk_time_interval => INTERVAL '1 day');
        INSERT INTO metrics(ts, tenant, value)
            VALUES (now() - INTERVAL '2 hours', 't1', 1.0),
                   (now() - INTERVAL '1 hour',  't1', 2.0),
                   (now(),                       't2', 3.0);

        -- ANALYZE so n_live_tup gets populated immediately.
        ANALYZE widgets;
        ANALYZE widget_events;
        ANALYZE metrics;
        "#,
    )
    .await
    .expect("seed");
}

async fn body_json(resp: axum::http::Response<Body>) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()))
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn overview_wire_shape_matches_frontend_reviver() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/overview")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    // Frontend reviver expects these exact field names — the test
    // is the wire contract.
    assert!(body.get("file_name").is_some(), "file_name missing");
    assert!(
        body.get("sqlite_version")
            .map(|v| v.is_null())
            .unwrap_or(false),
        "sqlite_version must be null on the wire"
    );
    assert!(body.get("size_on_disk").and_then(|v| v.as_str()).is_some());
    assert!(body.get("created").is_some());
    assert!(body.get("modified").is_some());
    assert!(body.get("tables").and_then(|v| v.as_i64()).unwrap() >= 2);
    for field in ["row_counts", "column_counts", "index_counts"] {
        assert!(
            body.get(field).and_then(|v| v.as_array()).is_some(),
            "{field} must be an array"
        );
    }
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn tables_lists_seeded_relations() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/tables")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let names: Vec<&str> = body["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"widgets"));
    assert!(names.contains(&"widget_events"));
    assert!(names.contains(&"metrics"));
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn table_detail_includes_synthesised_ddl() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/tables/widgets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "widgets");
    assert_eq!(body["column_count"], 3);
    let sql = body["sql"].as_str().unwrap();
    assert!(sql.contains("CREATE TABLE public.widgets"));
    assert!(sql.contains("name text NOT NULL"));
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn table_data_paginates() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/tables/widgets/data?page=1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["columns"][0], "id");
    assert_eq!(body["rows"].as_array().unwrap().len(), 3);
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn erd_emits_widget_to_widget_events_fk() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/erd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;

    let rels = body["relationships"].as_array().unwrap();
    let fk = rels
        .iter()
        .find(|r| r["from_table"] == "widget_events" && r["from_column"] == "widget_id")
        .expect("widget_events → widgets FK missing");
    assert_eq!(fk["to_table"], "widgets");
    assert_eq!(fk["to_column"], "id");

    // widgets.id should appear as a primary key column.
    let widgets = body["tables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == "widgets")
        .unwrap();
    let id_col = widgets["columns"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "id")
        .unwrap();
    assert_eq!(id_col["is_primary_key"], true);
}

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn autocomplete_returns_seeded_tables() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/autocomplete")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let names: Vec<&str> = body["tables"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["table_name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"widgets"));
}

// ---------------------------------------------------------- negative tests

#[tokio::test]
#[ignore = "requires docker / timescale image"]
async fn query_drop_table_is_rejected_and_table_survives() {
    let (client, _guard) = with_timescale().await;
    seed(client.pool()).await;
    let app = router(client.clone());

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/warehouse/explorer/query")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"sql":"DROP TABLE widgets"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    // 25006 read_only_sql_transaction → mapped to 400.
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    // Widgets still exists.
    let still_there: (i64,) = sqlx::query_as("SELECT count(*)::bigint FROM widgets")
        .fetch_one(client.pool())
        .await
        .expect("widgets table must still exist after rejected DROP");
    assert_eq!(still_there.0, 3);
}

#[tokio::test]
async fn malicious_table_identifier_is_rejected_before_db_round_trip() {
    // No container needed — the validator short-circuits.
    // We still need a router; use a connect-less stub pool by
    // building one against an obviously-unreachable address. The
    // request must never reach it.
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(std::time::Duration::from_millis(50))
        .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
        .expect("lazy pool");
    let client = starter_store_warehouse::WarehouseClient::from_pool(pool);
    let app = router(client);

    // `widgets; DROP TABLE widgets` — fails the identifier regex.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/warehouse/explorer/tables/widgets;%20DROP%20TABLE%20widgets")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = body_json(resp).await;
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("invalid table identifier"),
        "body = {body:?}"
    );
}
