//! Compose the `/auth/*` handler routes into one `Router` the
//! consumer merges via `ServerBuilder::merge_router`.
//!
//! The handlers close over `Arc<AuthState>` rather than threading
//! the auth state through axum's `State` extractor — this keeps the
//! router compatible with any consumer `AppState` without
//! state-type gymnastics.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::signup::mode::SignupMode;

use super::login::{handler as login_handler, LoginRequest};
use super::logout::handler as logout_handler;
use super::me::handler as me_handler;
use super::signup::{handler as signup_handler, SignupRequest};
use super::state::AuthState;

/// Extract client IP from `X-Forwarded-For` or `X-Real-Ip` headers.
/// Falls back to `0.0.0.0` when neither is present.
fn extract_ip(headers: &HeaderMap) -> IpAddr {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.trim().parse::<IpAddr>().ok())
        })
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}

/// Build the `/auth/{login,logout,me}` router, conditionally including
/// `/auth/signup` when signup is enabled.
///
/// Generic over the consumer's `AppState` so it merges into the
/// existing builder without adapters.
pub fn auth_router<S: Clone + Send + Sync + 'static>(state: AuthState) -> Router<S> {
    let state = Arc::new(state);

    let login_state = state.clone();
    let logout_state = state.clone();
    let me_state = state.clone();

    let mut router = Router::new()
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
        );

    // Conditionally mount signup when the mode is not Disabled (R9).
    if let SignupMode::Open { default_role } = state.signup.clone() {
        let signup_state = state.clone();
        router = router.route(
            "/auth/signup",
            post(move |headers: HeaderMap, body: Json<SignupRequest>| {
                let state = signup_state.clone();
                let ip = extract_ip(&headers);
                async move { signup_handler(state, ip, default_role, body).await }
            }),
        );
    }

    router
}
