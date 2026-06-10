//! The caller-context routes — identity context and the caller's preferences.

pub mod get;
pub mod preferences;
mod prefs_apply;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// `GET /api/v1/me` and `GET`/`PATCH /api/v1/me/preferences`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/me", get(get::get_me))
        .route(
            "/api/v1/me/preferences",
            get(preferences::get_me_preferences).patch(preferences::patch_me_preferences),
        )
}
