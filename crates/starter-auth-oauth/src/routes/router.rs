//! Compose the OAuth `start` and `callback` handlers into one
//! `Router` the consumer merges into its top-level app.
//!
//! Generic over the consumer's `AppState` so this composes with the
//! existing `ServerBuilder::merge_router` shape without forcing a
//! shared state type. The path segment `{provider}` is a real
//! captured parameter — a typo lands on axum's 404, not in a
//! runtime parse error inside a handler (SCOPE §"Routes").

use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::routing::get;
use axum::Router;

use super::callback::{handler as callback_handler, CallbackQuery};
use super::start::{handler as start_handler, StartQuery};
use super::state::OAuthRoutesState;

/// Build the `/auth/oauth/{provider}/{login,callback}` router.
///
/// `S` is the consumer's `AppState` type; we never read it because
/// the OAuth handlers close over [`OAuthRoutesState`]. Marker
/// trait bounds (`Clone + Send + Sync + 'static`) match the ones
/// `axum::Router` itself demands.
pub fn oauth_router<S: Clone + Send + Sync + 'static>(state: OAuthRoutesState) -> Router<S> {
    let state = Arc::new(state);
    let start_state = state.clone();
    let callback_state = state;

    Router::new()
        .route(
            "/auth/oauth/{provider}/login",
            get(move |Path(provider): Path<String>, Query(q): Query<StartQuery>| {
                let state = start_state.clone();
                async move { start_handler(state, provider, q).await }
            }),
        )
        .route(
            "/auth/oauth/{provider}/callback",
            get(
                move |Path(provider): Path<String>, Query(q): Query<CallbackQuery>| {
                    let state = callback_state.clone();
                    async move { callback_handler(state, provider, q).await }
                },
            ),
        )
}
