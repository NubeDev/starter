//! The caller-context route.

pub mod get;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// `GET /api/v1/me`.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/me", get(get::get_me))
}
