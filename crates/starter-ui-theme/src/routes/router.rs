//! Compose the six handlers into an `axum::Router`. The consumer
//! merges this into their app via `ServerBuilder::merge_router`
//! (or `axum::Router::merge`).
//!
//! Wrap with `starter_server::auth::with_principal` before mounting
//! so handler-level guards see the `Principal` extension. See the
//! module-level docs for the auth contract.

use std::sync::Arc;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;

use super::{
    delete_favicon, delete_logo, get_favicon, get_logo, get_theme, post_favicon, post_logo,
    put_theme, ThemeState,
};

/// Build the `/api/v1/ui/theme/*` router.
///
/// Generic over the consumer's `AppState` so it merges cleanly into
/// the existing axum stack without state-type gymnastics (same
/// pattern `starter_auth_users::auth_router` uses).
pub fn theme_router<S: Clone + Send + Sync + 'static>(state: ThemeState) -> Router<S> {
    let state = Arc::new(state);

    let s_get = state.clone();
    let s_put = state.clone();
    let s_logo_get = state.clone();
    let s_logo_post = state.clone();
    let s_logo_del = state.clone();
    let s_fav_get = state.clone();
    let s_fav_post = state.clone();
    let s_fav_del = state;

    Router::new()
        .route(
            "/api/v1/ui/theme",
            get(move |req: Request<Body>| {
                let s = s_get.clone();
                async move { get_theme(s, req).await }
            })
            .put(move |req: Request<Body>| {
                let s = s_put.clone();
                async move { put_theme(s, req).await }
            }),
        )
        .route(
            "/api/v1/ui/theme/logo",
            get(move || {
                let s = s_logo_get.clone();
                async move { get_logo(s).await }
            })
            .post(move |req: Request<Body>| {
                let s = s_logo_post.clone();
                async move { post_logo(s, req).await }
            })
            .delete(move |req: Request<Body>| {
                let s = s_logo_del.clone();
                async move { delete_logo(s, req).await }
            }),
        )
        .route(
            "/api/v1/ui/theme/favicon",
            get(move || {
                let s = s_fav_get.clone();
                async move { get_favicon(s).await }
            })
            .post(move |req: Request<Body>| {
                let s = s_fav_post.clone();
                async move { post_favicon(s, req).await }
            })
            .delete(move |req: Request<Body>| {
                let s = s_fav_del.clone();
                async move { delete_favicon(s, req).await }
            }),
        )
}
