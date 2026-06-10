//! Dashboard and panel routes. The slug is resolved to the immutable id at the
//! request edge; tenant scoping comes from the principal, RLS enforces it.

pub mod add_panel;
mod convert;
pub mod create;
pub mod delete;
pub mod delete_panel;
pub mod duplicate;
pub mod export;
pub mod get;
pub mod import;
pub mod list;
pub mod update;
pub mod update_panel;

use axum::routing::{delete, get as http_get, post};
use axum::Router;

use crate::state::AppState;

/// Dashboard collection/item routes plus the panel sub-routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/dashboards",
            post(create::create_dashboard).get(list::list_dashboards),
        )
        .route(
            "/api/v1/dashboards/import",
            post(import::import_dashboard),
        )
        .route(
            "/api/v1/dashboards/{slug}",
            http_get(get::get_dashboard)
                .patch(update::update_dashboard)
                .delete(delete::delete_dashboard),
        )
        .route(
            "/api/v1/dashboards/{slug}/export",
            http_get(export::export_dashboard),
        )
        .route(
            "/api/v1/dashboards/{slug}/duplicate",
            post(duplicate::duplicate_dashboard),
        )
        .route(
            "/api/v1/dashboards/{slug}/panels",
            post(add_panel::add_panel),
        )
        .route(
            "/api/v1/panels/{id}",
            delete(delete_panel::delete_panel).patch(update_panel::update_panel),
        )
}
