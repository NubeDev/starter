//! Compose the three `/auth/*` handler routes into one `Router` the
//! consumer merges via `ServerBuilder::merge_router`.
//!
//! The handlers close over `Arc<AuthState>` rather than threading
//! the auth state through axum's `State` extractor — this keeps the
//! router compatible with any consumer `AppState` without
//! state-type gymnastics.

use std::sync::Arc;

use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};

use super::login::{handler as login_handler, LoginRequest};
use super::logout::handler as logout_handler;
use super::me::handler as me_handler;
use super::state::AuthState;

/// Build the `/auth/{login,logout,me}` router.
///
/// Generic over the consumer's `AppState` so it merges into the
/// existing builder without adapters.
pub fn auth_router<S: Clone + Send + Sync + 'static>(state: AuthState) -> Router<S> {
    let state = Arc::new(state);

    let login_state = state.clone();
    let logout_state = state.clone();
    let me_state = state;

    Router::new()
        .route(
            "/auth/login",
            post(move |body: Json<LoginRequest>| {
                let state = login_state.clone();
                async move { login_handler(state, body).await }
            }),
        )
        .route(
            "/auth/logout",
            post(move |headers: HeaderMap| {
                let state = logout_state.clone();
                async move { logout_handler(state, headers).await }
            }),
        )
        .route(
            "/auth/me",
            get(move |headers: HeaderMap| {
                let state = me_state.clone();
                async move { me_handler(state, headers).await }
            }),
        )
}
