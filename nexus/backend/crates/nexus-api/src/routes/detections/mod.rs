//! Detection + findings routes (WS-15): the `/api/v1/detections/*` CRUD surface
//! and the `/api/v1/findings/*` browse + ack/resolve surface.

pub mod convert;
pub mod crud;
pub mod findings;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

/// The detections + findings surface.
pub fn router() -> Router<AppState> {
    Router::new()
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
