//! Alerting routes: rule CRUD, the event history, channels, and silences.

pub mod channels;
pub mod convert;
pub mod events;
pub mod rules;
pub mod silences;

use axum::routing::{delete, get};
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/alerts/*` surface.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/alerts/rules",
            get(rules::list_rules).post(rules::create_rule),
        )
        .route(
            "/api/v1/alerts/rules/{id}",
            get(rules::get_rule)
                .put(rules::update_rule)
                .delete(rules::delete_rule),
        )
        .route("/api/v1/alerts/events", get(events::list_events))
        .route(
            "/api/v1/alerts/channels",
            get(channels::list_channels).post(channels::create_channel),
        )
        .route(
            "/api/v1/alerts/channels/{id}",
            delete(channels::delete_channel),
        )
        .route(
            "/api/v1/alerts/silences",
            get(silences::list_silences).post(silences::create_silence),
        )
        .route(
            "/api/v1/alerts/silences/{id}",
            delete(silences::delete_silence),
        )
}
