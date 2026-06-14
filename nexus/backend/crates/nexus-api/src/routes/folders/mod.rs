//! Folder routes (WS-05). Folders organise dashboards into a nestable tree; they
//! key on an immutable id and re-root (never destroy) their contents on delete.

mod convert;
pub mod create;
pub mod delete;
pub mod list;
mod recorded;
pub mod update;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

/// Folder collection/item routes. There is no single-folder GET: the flat list
/// at `GET /folders` carries every field a client needs to assemble the tree.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/folders",
            post(create::create_folder).get(list::list_folders),
        )
        .route(
            "/api/v1/folders/{id}",
            axum::routing::patch(update::update_folder).delete(delete::delete_folder),
        )
}
