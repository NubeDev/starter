//! Compose the OAuth `start`, `callback`, `link`, `unlink`, and
//! `identities` handlers into one `Router` the consumer merges into
//! its top-level app.
//!
//! Generic over the consumer's `AppState` so this composes with the
//! existing `ServerBuilder::merge_router` shape without forcing a
//! shared state type. The path segment `{provider}` is a real
//! captured parameter — a typo lands on axum's 404, not in a
//! runtime parse error inside a handler (SCOPE §"Routes").

use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::http::HeaderMap;
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use super::callback::{handler as callback_handler, CallbackQuery};
use super::link::{handler as link_handler, LinkRequest};
use super::list::handler as list_handler;
use super::start::{handler as start_handler, StartQuery};
use super::state::OAuthRoutesState;
use super::unlink::handler as unlink_handler;

/// Build the `/auth/oauth/{provider}/{login,callback,link}` +
/// `/auth/oauth/{provider}` + `/auth/oauth/identities` router.
///
/// `S` is the consumer's `AppState` type; we never read it because
/// the OAuth handlers close over [`OAuthRoutesState`]. Marker
/// trait bounds (`Clone + Send + Sync + 'static`) match the ones
/// `axum::Router` itself demands.
pub fn oauth_router<S: Clone + Send + Sync + 'static>(state: OAuthRoutesState) -> Router<S> {
    let state = Arc::new(state);
    let start_state = state.clone();
    let callback_state = state.clone();
    let link_state = state.clone();
    let unlink_state = state.clone();
    let list_state = state;

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
        .route(
            "/auth/oauth/{provider}/link",
            post(
                move |Path(provider): Path<String>,
                      headers: HeaderMap,
                      body: Option<Json<LinkRequest>>| {
                    let state = link_state.clone();
                    async move { link_handler(state, provider, headers, body).await }
                },
            ),
        )
        .route(
            "/auth/oauth/{provider}",
            delete(
                move |Path(provider): Path<String>, headers: HeaderMap| {
                    let state = unlink_state.clone();
                    async move { unlink_handler(state, provider, headers).await }
                },
            ),
        )
        .route(
            "/auth/oauth/identities",
            get(move |headers: HeaderMap| {
                let state = list_state.clone();
                async move { list_handler(state, headers).await }
            }),
        )
}
