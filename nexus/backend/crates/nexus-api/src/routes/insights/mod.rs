//! Stored-insight routes (RW-06). CRUD for the named post-query transform
//! scripts a panel references; tenant-scoped and RLS-isolated like folders. Every
//! save compiles the script first, so a persisted insight is always runnable.

mod convert;
pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;
mod validate;

use axum::routing::get as http_get;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/insights` collection + item routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/insights",
            http_get(list::list_insights).post(create::create_insight),
        )
        .route(
            "/api/v1/insights/{id}",
            http_get(get::get_insight)
                .patch(update::update_insight)
                .delete(delete::delete_insight),
        )
}
