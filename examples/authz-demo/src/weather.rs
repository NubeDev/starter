//! Host-side wiring for the `com.acme.weather` extension's two REST
//! endpoints.
//!
//! In a fully-featured deployment these routes would be mounted by
//! `starter_ext_server::rest_router` straight from the extension's
//! `contributes.rest[]` block. The demo wires them by hand instead
//! so each route can carry its own policy-engine gate
//! (`with_permission`) — the manifest still drives discovery (the
//! `/extensions/*` admin slice lists the extension), but the host
//! decides the per-route authz contract.
//!
//! What this proves end-to-end:
//!   - the extension shows up in `/extensions` (manifest-driven),
//!   - `GET  /weather/forecast` is gated by `("weather", "read")`,
//!   - `POST /weather/refresh`  is gated by `("weather", "refresh")`.

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde_json::{json, Value};
use starter_authz::with_permission;

pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    // GET /weather/forecast — read. Gated by ("weather", "read").
    let read = Router::new().route("/weather/forecast", get(forecast));
    let read = with_permission(read, "weather", "read");

    // POST /weather/refresh — read/write. Gated by ("weather", "refresh").
    let write = Router::new().route("/weather/refresh", post(refresh));
    let write = with_permission(write, "weather", "refresh");

    Router::new().merge(read).merge(write)
}

async fn forecast() -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "city": "Brisbane",
        "temp_c": 24.5,
        "condition": "sunny"
    })))
}

async fn refresh() -> Result<Json<Value>, (StatusCode, String)> {
    Ok(Json(json!({
        "refreshed_at": Utc::now().to_rfc3339(),
        "cleared": 0
    })))
}
