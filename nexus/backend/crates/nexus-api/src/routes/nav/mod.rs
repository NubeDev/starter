//! Navigation-tree routes (WS-13). The nav tree is the single navigation +
//! access surface: each node mounts a dashboard page (with context) or a static
//! route, and `view` on a node is what gates navigating to it.

mod convert;
pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

use axum::routing::get as http_get;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/nav` surface: list (access-filtered) + create on the collection,
/// get/update/delete on a node.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/nav",
            http_get(list::list_nav).post(create::create_nav),
        )
        .route(
            "/api/v1/nav/{id}",
            http_get(get::get_nav)
                .patch(update::update_nav)
                .delete(delete::delete_nav),
        )
}
