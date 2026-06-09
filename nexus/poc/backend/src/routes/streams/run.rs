//! POST /api/streams/run — run the config (bounded) and return collected rows.

use axum::Json;

use crate::dto::stream::{RunRequest, RunResponse};
use crate::engine;

pub async fn run(Json(req): Json<RunRequest>) -> Json<RunResponse> {
    let outcome = engine::run_config(req.config, req.timeout_ms).await;
    Json(RunResponse {
        ok: outcome.error.is_none(),
        error: outcome.error,
        row_count: outcome.rows.len(),
        rows: outcome.rows,
        cancelled: outcome.cancelled,
    })
}
