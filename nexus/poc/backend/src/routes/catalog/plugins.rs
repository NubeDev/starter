//! GET /api/plugins — list every registered component and its source.

use axum::Json;

use crate::engine::catalog::plugins::{list as plugin_list, PluginEntry};

pub async fn list() -> Json<Vec<PluginEntry>> {
    Json(plugin_list())
}
