//! Datasource CRUD routes. Each handler resolves the caller's tenant and
//! delegates to the tenant-scoped store; the secret never crosses the wire.

mod convert;
pub mod create;
pub mod delete;
pub mod get;
pub mod kinds;
pub mod list;
pub mod probe_outcome;
pub mod query;
pub mod schema;
pub mod test;
pub mod test_connection;
pub mod update;

use axum::routing::{get as http_get, post};
use axum::Router;

use crate::state::AppState;

/// `/api/v1/datasources` collection and `/api/v1/datasources/:id` item routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/datasources",
            post(create::create_datasource).get(list::list_datasources),
        )
        .route(
            "/api/v1/datasources/test",
            post(test_connection::test_connection),
        )
        // Declared before the `:id` item route so `kinds` is matched as the
        // catalogue, never parsed as a datasource id.
        .route(
            "/api/v1/datasources/kinds",
            http_get(kinds::list_datasource_kinds),
        )
        .route(
            "/api/v1/datasources/{id}",
            http_get(get::get_datasource)
                .put(update::update_datasource)
                .delete(delete::delete_datasource),
        )
        .route(
            "/api/v1/datasources/{id}/query",
            post(query::query_datasource),
        )
        .route(
            "/api/v1/datasources/{id}/schema",
            http_get(schema::datasource_schema),
        )
        .route("/api/v1/datasources/{id}/test", post(test::test_datasource))
}
