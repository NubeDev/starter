//! Saved-flow routes: CRUD plus start/stop of the running stream.

pub mod convert;
pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod start;
pub mod stop;
pub mod update;

use axum::routing::{get as http_get, post};
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/flows` surface.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/flows",
            http_get(list::list_flows).post(create::create_flow),
        )
        .route(
            "/api/v1/flows/{id}",
            http_get(get::get_flow)
                .put(update::update_flow)
                .delete(delete::delete_flow),
        )
        .route("/api/v1/flows/{id}/start", post(start::start_flow))
        .route("/api/v1/flows/{id}/stop", post(stop::stop_flow))
}
