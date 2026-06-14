//! Liveness and readiness probes (WS-16).
//!
//! `starter_server::ServerBuilder` already mounts a dumb `/health` that returns
//! 200 unconditionally — fine as a load-balancer target, but it cannot tell a
//! wedged tokio runtime from a healthy-but-slow one. These two routes add that
//! resolution:
//!
//! - **`GET /livez`** reads the runtime canary. If the canary atomic has not
//!   advanced within [`STALENESS_BUDGET`], the runtime is wedged (every worker
//!   parked) and we return 503 so an orchestrator restarts the process. A
//!   transient stop-the-world stays under the budget and reports 200.
//! - **`GET /readyz`** is liveness **and** a `SELECT 1` against the metadata
//!   pool within a short timeout — "can this process actually serve a request
//!   that touches its database right now." Returns 503 if the runtime is stale
//!   or the DB probe fails/times out.
//!
//! Both are unauthenticated: they read no `Principal` and touch no tenant data,
//! so they merge into the product router and are reachable without a token.

use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;

use crate::boot::runtime_canary::STALENESS_BUDGET;
use crate::state::AppState;

/// How long the `/readyz` `SELECT 1` may take before the DB is declared not
/// ready. Short — readiness should fail fast so a struggling replica is pulled
/// from rotation rather than hanging the probe.
const DB_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// `GET /livez` — is the tokio runtime advancing?
pub async fn livez(State(state): State<AppState>) -> impl IntoResponse {
    match state.canary.staleness() {
        Some(s) if s > STALENESS_BUDGET => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "wedged",
                "canary_staleness_seconds": s.as_secs(),
                "staleness_budget_seconds": STALENESS_BUDGET.as_secs(),
            })),
        ),
        Some(s) => (
            StatusCode::OK,
            Json(json!({
                "status": "live",
                "canary_staleness_seconds": s.as_secs(),
            })),
        ),
        // Clock skew (atomic holds a future timestamp): the runtime plainly just
        // stored a tick, so it is live.
        None => (
            StatusCode::OK,
            Json(json!({ "status": "live", "canary_staleness_seconds": 0 })),
        ),
    }
}

/// `GET /readyz` — runtime live **and** the metadata DB answers a trivial query.
pub async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    if state.canary.is_stale() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "reason": "runtime_wedged" })),
        );
    }

    let probe = tokio::time::timeout(
        DB_PROBE_TIMEOUT,
        sqlx::query_scalar::<_, i32>("SELECT 1").fetch_one(&state.metadata),
    )
    .await;

    match probe {
        Ok(Ok(_)) => (StatusCode::OK, Json(json!({ "status": "ready" }))),
        Ok(Err(e)) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "reason": "db_error", "detail": e.to_string() })),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "not_ready", "reason": "db_timeout" })),
        ),
    }
}

/// `GET /livez` and `GET /readyz`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
}
