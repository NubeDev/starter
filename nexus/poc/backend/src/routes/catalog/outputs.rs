//! GET /api/outputs — list the configurable output types.

use axum::Json;

use crate::dto::catalog::ComponentKind;
use crate::engine::catalog;

pub async fn list() -> Json<Vec<ComponentKind>> {
    Json(catalog::outputs::list())
}
