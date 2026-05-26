//! Axum handlers for the seven explorer endpoints.
//!
//! The router built by [`routes`] is mounted under
//! [`crate::MOUNT_PREFIX`] (`/api/warehouse/explorer`) by both
//! [`crate::router`] and [`crate::router_with_auth`].

use std::time::{Duration, Instant};

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

use crate::queries;
use crate::types::QueryRequest;
use crate::validate::is_safe_identifier;
use crate::{CachedAutocomplete, ExplorerState};

/// `/autocomplete` cache lifetime. Short enough that newly created
/// tables show up within a single Monaco mount cycle, long enough
/// to flatten the stampede.
const AUTOCOMPLETE_TTL: Duration = Duration::from_secs(5);

pub fn routes() -> Router<ExplorerState> {
    Router::new()
        .route("/overview", get(overview))
        .route("/tables", get(tables))
        .route("/tables/{name}", get(table_detail))
        .route("/tables/{name}/data", get(table_data))
        .route("/query", post(run_query))
        .route("/autocomplete", get(autocomplete))
        .route("/erd", get(erd))
}

// ---------------------------------------------------------------- helpers

fn sql_error(err: sqlx::Error) -> axum::response::Response {
    warn!(target: "warehouse.explorer", error = %err, "sqlx error");
    let status = match &err {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        sqlx::Error::Database(db) => match db.code().as_deref() {
            // 25006 read_only_sql_transaction — the explicit gate
            // for POST /query rejecting mutations.
            Some("25006") => StatusCode::BAD_REQUEST,
            // 57014 query_canceled — statement_timeout fired.
            Some("57014") => StatusCode::REQUEST_TIMEOUT,
            // 42xxx — syntax / undefined-table / privilege errors
            // are user input problems, surface as 400 rather than
            // 500 so the explorer renders the message inline.
            Some(c) if c.starts_with("42") => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(json!({ "error": err.to_string() }))).into_response()
}

fn invalid_identifier(name: &str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "error": format!("invalid table identifier: {name:?}"),
        })),
    )
        .into_response()
}

// ---------------------------------------------------------------- handlers

async fn overview(State(state): State<ExplorerState>) -> axum::response::Response {
    match queries::overview(state.client.pool()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(e) => sql_error(e),
    }
}

async fn tables(State(state): State<ExplorerState>) -> axum::response::Response {
    match queries::tables(state.client.pool()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(e) => sql_error(e),
    }
}

async fn table_detail(
    State(state): State<ExplorerState>,
    Path(name): Path<String>,
) -> axum::response::Response {
    if !is_safe_identifier(&name) {
        return invalid_identifier(&name);
    }
    match queries::table_detail(state.client.pool(), &name).await {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("table not found: {name}") })),
        )
            .into_response(),
        Err(e) => sql_error(e),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct PageQuery {
    #[serde(default = "default_page")]
    pub page: i64,
}

fn default_page() -> i64 {
    1
}

async fn table_data(
    State(state): State<ExplorerState>,
    Path(name): Path<String>,
    Query(q): Query<PageQuery>,
) -> axum::response::Response {
    if !is_safe_identifier(&name) {
        return invalid_identifier(&name);
    }
    let page = q.page.max(1);
    match queries::table_data(state.client.pool(), &name, page).await {
        Ok(payload) => Json(payload).into_response(),
        Err(e) => sql_error(e),
    }
}

async fn run_query(
    State(state): State<ExplorerState>,
    Json(body): Json<QueryRequest>,
) -> axum::response::Response {
    if body.sql.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "sql is empty" })),
        )
            .into_response();
    }
    match queries::run_query(state.client.pool(), &body.sql).await {
        Ok(payload) => Json(payload).into_response(),
        Err(e) => sql_error(e),
    }
}

async fn autocomplete(State(state): State<ExplorerState>) -> axum::response::Response {
    {
        let cache = state.autocomplete_cache.lock().await;
        if let Some(entry) = cache.as_ref() {
            if entry.fetched_at.elapsed() < AUTOCOMPLETE_TTL {
                return Json(entry.payload.clone()).into_response();
            }
        }
    }
    match queries::autocomplete(state.client.pool()).await {
        Ok(payload) => {
            let mut cache = state.autocomplete_cache.lock().await;
            *cache = Some(CachedAutocomplete {
                fetched_at: Instant::now(),
                payload: payload.clone(),
            });
            Json(payload).into_response()
        }
        Err(e) => sql_error(e),
    }
}

async fn erd(State(state): State<ExplorerState>) -> axum::response::Response {
    match queries::erd(state.client.pool()).await {
        Ok(payload) => Json(payload).into_response(),
        Err(e) => sql_error(e),
    }
}
