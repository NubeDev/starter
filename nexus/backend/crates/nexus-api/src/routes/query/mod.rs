//! The one-shot query route.

pub mod run;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub use run::run_query;

/// `POST /api/v1/query`. Mounted into the app router by the binary.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/query", post(run_query))
}
