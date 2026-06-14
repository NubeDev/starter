//! Detection + findings routes: the `/api/v1/detections/*` CRUD surface, the
//! `/api/v1/findings/*` browse + ack/resolve surface, and — for "alert-type"
//! detections — the notification channels/silences/events surface ([`notify`]).

pub mod convert;
pub mod crud;
pub mod findings;
pub mod notify;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// The detections + findings + notification surface.
pub fn router() -> Router<AppState> {
    Router::new()
        // Notification config (static segments registered before `{id}`).
        .route(
            "/api/v1/detections/channels",
            get(notify::list_channels).post(notify::create_channel),
        )
        .route(
            "/api/v1/detections/channels/{id}",
            axum::routing::delete(notify::delete_channel),
        )
        .route(
            "/api/v1/detections/silences",
            get(notify::list_silences).post(notify::create_silence),
        )
        .route(
            "/api/v1/detections/silences/{id}",
            axum::routing::delete(notify::delete_silence),
        )
        .route(
            "/api/v1/detections/notify-events",
            get(notify::list_notify_events),
        )
        .route(
            "/api/v1/detections",
            get(crud::list_detections).post(crud::create_detection),
        )
        .route(
            "/api/v1/detections/{id}",
            get(crud::get_detection)
                .put(crud::update_detection)
                .delete(crud::delete_detection),
        )
        .route("/api/v1/detections/{id}/run", post(crud::run_now))
        .route("/api/v1/detections/{id}/stats", get(crud::get_stats))
        .route("/api/v1/findings", get(findings::list_findings))
        .route("/api/v1/findings/{id}", get(findings::get_finding))
        .route("/api/v1/findings/{id}/ack", post(findings::ack_finding))
        .route(
            "/api/v1/findings/{id}/resolve",
            post(findings::resolve_finding),
        )
}
