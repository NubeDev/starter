//! POST /api/streams/validate — build the config without running it.

use axum::Json;

use crate::dto::stream::{StreamRequest, ValidateResponse};
use crate::engine;

pub async fn validate(Json(req): Json<StreamRequest>) -> Json<ValidateResponse> {
    let error = engine::validate(req.config);
    Json(ValidateResponse {
        ok: error.is_none(),
        error,
    })
}
