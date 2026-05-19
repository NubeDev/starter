//! Compose the single `/auth/claim` route into a `Router` the
//! consumer merges via `ServerBuilder::merge_router`.

use axum::{routing::post, Router};

use super::claim::{handler, ClaimState};

/// Build the `POST /auth/claim` router.
///
/// The router is generic over the consumer's `AppState`; the claim
/// store is bound as nested state so consumers don't have to thread
/// it through their own state struct.
pub fn claim_router<S: Clone + Send + Sync + 'static>(store: ClaimState) -> Router<S> {
    Router::new()
        .route("/auth/claim", post(handler))
        .with_state(store)
}
