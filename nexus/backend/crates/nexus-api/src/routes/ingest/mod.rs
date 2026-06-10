//! Push-ingest route: feed JSON into a running `http_ingest` flow.

pub mod push;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

/// The `/api/v1/ingest` surface — a single push endpoint keyed by flow id.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/ingest/{flow_id}", post(push::push_ingest))
}
