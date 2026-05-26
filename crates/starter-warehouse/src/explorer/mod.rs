//! ClickHouse explorer sub-router.
//!
//! Read-only HTTP surface mounted at `/api/warehouse/ch/*` by
//! [`crate::lib`]. Hands every request to the typed `ChClient` from
//! `starter-store-warehouse` — no second connection, no raw write
//! path. Authz (`warehouse.read`) is applied at the mount site in
//! `lib.rs`, not here, so the router stays composable.
//!
//! Design notes:
//! `rubix/docs/design/warehouse/explorer/README.md`.
//!
//! Public surface:
//!
//! ```text
//! GET  /api/warehouse/ch/overview
//! GET  /api/warehouse/ch/tables
//! GET  /api/warehouse/ch/tables/{name}
//! GET  /api/warehouse/ch/tables/{name}/data
//! GET  /api/warehouse/ch/tables/{name}/columns
//! GET  /api/warehouse/ch/erd
//! GET  /api/warehouse/ch/autocomplete
//! POST /api/warehouse/ch/query   { sql }   -- read-only allow-list
//! ```

pub mod parse;
pub mod queries;
pub mod types;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use starter_store_warehouse::{ChClient, ChClientError};

use self::parse::{Reject, Verdict};

/// Build the explorer sub-router. The returned `Router` is already
/// stateful (`with_state(ch)`) so callers can `.merge(...)` it into
/// a router with any (or no) outer state.
pub fn routes(ch: ChClient) -> Router {
    Router::new()
        .route("/api/warehouse/ch/overview", get(overview_handler))
        .route("/api/warehouse/ch/tables", get(tables_handler))
        .route("/api/warehouse/ch/tables/{name}", get(table_handler))
        .route(
            "/api/warehouse/ch/tables/{name}/data",
            get(table_data_handler),
        )
        .route(
            "/api/warehouse/ch/tables/{name}/columns",
            get(table_columns_handler),
        )
        .route("/api/warehouse/ch/erd", get(erd_handler))
        .route("/api/warehouse/ch/autocomplete", get(autocomplete_handler))
        .route("/api/warehouse/ch/query", post(query_handler))
        .with_state(ch)
}

fn map_err(err: ChClientError) -> (StatusCode, Json<serde_json::Value>) {
    tracing::warn!(target: "starter_warehouse::explorer", error = %err, "explorer query failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": "clickhouse_query_failed", "message": err.to_string()})),
    )
}

async fn overview_handler(State(ch): State<ChClient>) -> impl IntoResponse {
    let database = ch.config().database.clone();
    match queries::overview(&ch, &database).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

async fn tables_handler(State(ch): State<ChClient>) -> impl IntoResponse {
    match queries::tables(&ch).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

async fn table_handler(State(ch): State<ChClient>, Path(name): Path<String>) -> impl IntoResponse {
    match queries::table(&ch, &name).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

#[derive(Deserialize)]
struct PageQuery {
    #[serde(default = "default_page")]
    page: i64,
}

fn default_page() -> i64 {
    1
}

async fn table_data_handler(
    State(ch): State<ChClient>,
    Path(name): Path<String>,
    Query(q): Query<PageQuery>,
) -> impl IntoResponse {
    match queries::table_data(&ch, &name, q.page).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

async fn table_columns_handler(
    State(ch): State<ChClient>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    match queries::columns(&ch, &name).await {
        Ok(v) => Json(serde_json::json!({ "columns": v })).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

async fn erd_handler(State(ch): State<ChClient>) -> impl IntoResponse {
    match queries::erd(&ch).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

async fn autocomplete_handler(State(ch): State<ChClient>) -> impl IntoResponse {
    match queries::tables_with_columns(&ch).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}

#[derive(Deserialize)]
struct QueryBody {
    sql: String,
}

async fn query_handler(
    State(ch): State<ChClient>,
    Json(body): Json<QueryBody>,
) -> impl IntoResponse {
    match parse::classify(&body.sql) {
        Verdict::Allow => {}
        Verdict::Reject(reason) => {
            let msg = match reason {
                Reject::Empty => "empty SQL statement",
                Reject::NotReadOnly => "read-only endpoint; use rubix.warehouse.* verbs for writes",
            };
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": reason.as_str(), "message": msg })),
            )
                .into_response();
        }
    }
    match queries::query(&ch, &body.sql).await {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => map_err(e).into_response(),
    }
}
