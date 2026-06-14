//! GET /api/processors — list the configurable processor types.

use axum::Json;

use crate::dto::catalog::ComponentKind;
use crate::engine::catalog;

pub async fn list() -> Json<Vec<ComponentKind>> {
    Json(catalog::processors::list())
}
