//! The one-shot query route and the per-user query-history routes.

pub mod history;
pub mod run;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub use history::{list_query_history, star_query_history};
pub use run::run_query;

/// `POST /api/v1/query` plus the query-history surface. Mounted into the app
/// router by the binary.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/query", post(run_query))
        .route("/api/v1/query-history", get(list_query_history))
        .route(
            "/api/v1/query-history/{id}/star",
            post(star_query_history),
        )
}
