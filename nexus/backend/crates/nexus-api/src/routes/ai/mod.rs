//! AI assist routes: synchronous, task-typed assistance (vs. the streaming agent
//! sessions under `/agents`). One endpoint for now — `POST /ai/assist`.

pub mod assist;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/ai` surface.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/ai/assist", post(assist::ai_assist))
}
