//! Compose the three `/auth/*` handler routes into one Router the
//! consumer merges via `ServerBuilder::merge_router`.

use axum::Router;

/// Build the `/auth/{login,logout,me}` router.
///
/// Generic over the consumer's `AppState` so it merges into the
/// existing builder without adapters.
pub fn auth_router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        // .route("/auth/login", post(super::login::handler))
        // .route("/auth/logout", post(super::logout::handler))
        // .route("/auth/me", get(super::me::handler))
        // TODO(ap): wire handlers once their bodies land. Empty
        // router keeps the public seam usable today.
}
