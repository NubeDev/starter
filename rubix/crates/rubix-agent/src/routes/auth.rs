//! `/auth/*` — barrel re-export over `starter-auth-users`.
//!
//! Per the verb-per-file rule, the rubix binary never re-implements
//! handler logic from upstream starter crates. This module wraps
//! [`starter_auth_users::routes::auth_router`] so the binary's
//! `main.rs` mounts the routes through its own
//! [`Router`](axum::Router) composition site rather than reaching
//! across the `routes::` module boundary into another crate. See
//! [docs/design/auth/](../../../docs/design/auth/README.md).

use axum::Router;

use starter_auth_users::routes::{auth_router as upstream_auth_router, AuthState};

/// Build the `/auth/{login,logout,me}` router. The binary nests
/// this under `/api/v1` so the final mount points are
/// `/api/v1/auth/{login,logout,me}` (and `/api/v1/auth/signup`
/// when [`AuthState::with_signup_open`] is set — the rubix binary
/// does not enable signup; operator accounts come from
/// `rubix-admin bootstrap-user`).
pub fn auth_router(state: AuthState) -> Router {
    upstream_auth_router::<()>(state)
}
