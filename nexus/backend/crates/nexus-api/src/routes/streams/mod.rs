//! Live-stream routes: create a subscription, then connect its SSE feed.

pub mod create;
pub mod pending;
pub mod subscribe;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// `POST /api/v1/streams` and `GET /api/v1/streams/:id`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/streams", post(create::create_stream))
        .route("/api/v1/streams/{id}", get(subscribe::subscribe_stream))
}
