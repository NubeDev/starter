//! GET /api/inputs — list the configurable input types.

use axum::Json;

use crate::dto::catalog::ComponentKind;
use crate::engine::catalog;

pub async fn list() -> Json<Vec<ComponentKind>> {
    Json(catalog::inputs::list())
}
