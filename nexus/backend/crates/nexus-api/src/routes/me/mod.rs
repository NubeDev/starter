//! The caller-context routes — identity context, the caller's preferences, and
//! the caller's freeform settings bag.

pub mod get;
pub mod preferences;
mod prefs_apply;
pub mod settings;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

/// `GET /api/v1/me`, `GET`/`PATCH /api/v1/me/preferences`, and `GET`/`PUT
/// /api/v1/me/settings`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/me", get(get::get_me))
        .route(
            "/api/v1/me/preferences",
            get(preferences::get_me_preferences).patch(preferences::patch_me_preferences),
        )
        .route(
            "/api/v1/me/settings",
            get(settings::get_me_settings).put(settings::set_me_settings),
        )
}
