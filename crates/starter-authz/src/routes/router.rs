//! Compose the `/v1/authz/*` admin router. Wraps every route in
//! an Admin-role guard so a single misconfigured permission rule
//! cannot lock the admin out of fixing the policy table that
//! produced it. SCOPE.md "Admin authz routes are role-gated".

use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Extension, Router};
use starter_spi::auth::{Principal, Role};

use super::assignments::{create_assignment, delete_assignment, list_assignments};
use super::check::check_handler;
use super::resources::list_resources;
use super::rules::{create_rule, delete_rule, list_rules, update_rule};
use super::state::AuthzRoutesState;

/// Build the admin router. Generic over the consumer's `AppState`
/// so it merges into the existing server router unchanged.
pub fn authz_router<S>(state: AuthzRoutesState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = Arc::new(state);

    Router::<S>::new()
        .route("/v1/authz/rules", get(list_rules).post(create_rule))
        .route("/v1/authz/rules/{id}", put(update_rule).delete(delete_rule))
        .route(
            "/v1/authz/assignments",
            get(list_assignments).post(create_assignment),
        )
        .route("/v1/authz/assignments/{id}", delete(delete_assignment))
        .route("/v1/authz/resources", get(list_resources))
        .route("/v1/authz/check", post(check_handler))
        .layer(Extension(state))
        .layer(from_fn(admin_gate))
}

/// Inline copy of the role-rank guard from
/// `starter-server::auth::with_role`. Reproduced here so this
/// crate does not depend upward on `starter-server`.
async fn admin_gate(req: Request<Body>, next: Next) -> Response {
    let principal = match req.extensions().get::<Principal>() {
        Some(p) => p.clone(),
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };
    if principal.role == Role::Admin {
        next.run(req).await
    } else {
        StatusCode::FORBIDDEN.into_response()
    }
}

/// Verify the double-submit CSRF cookie / header pair. Returns
/// `Err(403)` on mismatch — handlers `?`-propagate this before
/// touching the store. Mirrors `starter_auth_users::routes`'s
/// logout/signup pattern.
#[allow(clippy::result_large_err)] // `axum::Response` size is fixed by axum.
pub(super) fn check_csrf(headers: &HeaderMap) -> Result<(), Response> {
    let cookies = parse_cookies(headers);
    let cookie = cookies.get("starter_csrf").map(String::as_str);
    let header = headers.get("x-csrf-token").and_then(|v| v.to_str().ok());
    match (cookie, header) {
        (Some(c), Some(h)) if c == h => Ok(()),
        _ => Err(StatusCode::FORBIDDEN.into_response()),
    }
}

fn parse_cookies(headers: &HeaderMap) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for value in headers.get_all(axum::http::header::COOKIE) {
        if let Ok(s) = value.to_str() {
            for pair in s.split(';') {
                if let Some((k, v)) = pair.trim().split_once('=') {
                    out.insert(k.to_string(), v.to_string());
                }
            }
        }
    }
    out
}
