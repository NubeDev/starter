//! GET /api/buffers — list the configurable buffer types.

use axum::Json;

use crate::dto::catalog::ComponentKind;
use crate::engine::catalog;

pub async fn list() -> Json<Vec<ComponentKind>> {
    Json(catalog::buffers::list())
}
