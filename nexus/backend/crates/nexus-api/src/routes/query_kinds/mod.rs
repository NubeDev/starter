//! Tenant-authored query-kind routes: CRUD for the named SQL queries an admin
//! promotes from Explore (WS-10 §4.5c).
//!
//! Distinct from the read-only picker catalogue at `/api/v1/query/kinds`, which
//! unions the built-in file pack with these and hides each kind's SQL. This
//! management surface is hyphenated (`/api/v1/query-kinds`) to avoid colliding
//! with that catalogue path, and returns the full authoring detail incl. `sql`.
//! Every save runs the same load-time lint a file kind passes, so a persisted
//! kind is always lint-clean.

pub mod convert;
pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

use axum::routing::get as http_get;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/query-kinds` surface.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/query-kinds",
            http_get(list::list_query_kinds_admin).post(create::create_query_kind),
        )
        .route(
            "/api/v1/query-kinds/{id}",
            http_get(get::get_query_kind)
                .put(update::update_query_kind)
                .delete(delete::delete_query_kind),
        )
}
