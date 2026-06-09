//! POST /api/sql/query — run DataFusion SQL over an inline JSON dataset.

use axum::Json;

use crate::dto::sql::{SqlRequest, SqlResponse};
use crate::engine;

pub async fn query(Json(req): Json<SqlRequest>) -> Json<SqlResponse> {
    let outcome = engine::sql_query(&req.query, &req.rows).await;
    Json(SqlResponse {
        ok: outcome.error.is_none(),
        error: outcome.error,
        row_count: outcome.rows.len(),
        rows: outcome.rows,
    })
}
