//! `POST /auth/login`. Body: `{ email, password }`. On success: sets
//! the session cookie + a non-httpOnly CSRF cookie, returns the CSRF
//! token in the response body so SPAs can pick it up without parsing
//! `Set-Cookie` headers.

use std::sync::Arc;

use axum::http::header::{HeaderValue, SET_COOKIE};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::password;
use crate::role::Role;
use crate::session::{issue_for_tenant, IssuedSession, SESSION_COOKIE};

use super::state::AuthState;

/// Cookie name carrying the CSRF double-submit token. Non-httpOnly on
/// purpose so the SPA can read it and echo it back as
/// `X-CSRF-Token`.
pub const CSRF_COOKIE: &str = "starter_csrf";

/// Request body for `POST /auth/login`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    /// User's email (the primary identifier in `starter_auth_users`).
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Response body for `POST /auth/login`.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    /// CSRF double-submit token. Send back as `X-CSRF-Token` on
    /// mutating cookie-authenticated requests.
    pub csrf_token: String,
}

/// Body returned by `POST /auth/login` when the matched user has no
/// local password (`password_hash IS NULL`). The SPA reads
/// `providers` to render "Sign in with GitHub / Google" buttons
/// without a guess-and-check round trip.
#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordNotSetResponse {
    /// Always `"password_not_set"`. Discriminator field; lets clients
    /// pattern-match without inspecting the HTTP status alone.
    pub error: &'static str,
    /// Provider ids the user has linked. Empty list when no
    /// third-party path is configured (the default
    /// [`crate::NoLinkedProviders`] impl).
    pub providers: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/auth/login",
    tag = "auth",
    operation_id = "login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Logged in; session + CSRF cookies set", body = LoginResponse),
        (status = 400, description = "Account has no local password; sign in via a linked provider", body = PasswordNotSetResponse),
        (status = 401, description = "Invalid credentials"),
    ),
)]
pub(crate) async fn handler(state: Arc<AuthState>, Json(body): Json<LoginRequest>) -> Response {
    let user = match state.users.find_by_email(&body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "login store lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    // `password_hash IS NULL` means this account was created via a
    // third-party sign-in path and never set a local password. Return
    // 400 (not 401) with the providers list so the SPA can route the
    // user to the right "Sign in with ..." button. The list is
    // produced by the `LinkedProvidersLookup` trait — the
    // `NoLinkedProviders` default returns `[]`, which is still a
    // valid response shape.
    let password_hash = match &user.password_hash {
        Some(h) => h.as_str(),
        None => {
            let providers = match state.linked_providers.linked_providers(&user.id).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!(
                        target: "starter_auth_users",
                        error = %e,
                        "linked-providers lookup failed",
                    );
                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };
            return (
                StatusCode::BAD_REQUEST,
                Json(PasswordNotSetResponse {
                    error: "password_not_set",
                    providers,
                }),
            )
                .into_response();
        }
    };
    match password::verify(&body.password, password_hash) {
        Ok(true) => {}
        Ok(false) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "login password verify failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // Resolve the tenant this session binds to. Mirrors the
    // `POST /auth/token` resolution rules (see `token::resolve_tenant`)
    // so cookie- and bearer-authenticated callers see the same
    // principal `tenant_id` on identical inputs:
    //   * 0 memberships + Admin role  -> "*" (super-admin sentinel)
    //   * 0 memberships + non-admin   -> NULL (downstream tenant-
    //                                     scoped routes reject with
    //                                     `no_tenant_binding`)
    //   * 1 membership                -> that tenant
    //   * N>1 memberships + Admin     -> "*"
    //   * N>1 memberships + non-admin -> first by created_at
    //                                     (picker UX is future work)
    //   * tenants store unwired       -> NULL (legacy single-tenant
    //                                     deployments rely on
    //                                     downstream defaults)
    let bound_tenant = resolve_login_tenant(&state, &user.id, &user.role).await;

    let issued: IssuedSession =
        match issue_for_tenant(state.sessions.as_ref(), &user.id, bound_tenant.as_deref()).await {
            Ok(i) => i,
            Err(e) => {
                tracing::warn!(target: "starter_auth_users", error = %e, "session issue failed");
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let session_cookie = format!(
        "{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax",
        value = issued.cookie_value,
    );
    let csrf_cookie = format!(
        "{CSRF_COOKIE}={value}; Path=/; SameSite=Lax",
        value = issued.csrf_token,
    );

    let mut resp = (
        StatusCode::OK,
        Json(LoginResponse {
            csrf_token: issued.csrf_token.clone(),
        }),
    )
        .into_response();
    let headers = resp.headers_mut();
    if let Ok(v) = HeaderValue::from_str(&session_cookie) {
        headers.append(SET_COOKIE, v);
    }
    if let Ok(v) = HeaderValue::from_str(&csrf_cookie) {
        headers.append(SET_COOKIE, v);
    }
    resp
}

/// Decide which tenant a freshly-issued cookie session binds to.
/// Returns `None` if the session should remain unbound (the user
/// has no memberships and is not a global Admin, or the tenants
/// store is not wired). See the call site for the full table.
async fn resolve_login_tenant(state: &AuthState, user_id: &str, role: &Role) -> Option<String> {
    let tenants = state.tenants.as_ref()?;
    let memberships = match tenants.memberships_for_user(user_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_users",
                error = %e,
                "memberships lookup failed during login; issuing unbound session",
            );
            return None;
        }
    };
    if matches!(role, Role::Admin) {
        // Global admins always get the super-admin sentinel so they
        // see every tenant's resources. Cookie + bearer paths agree.
        return Some("*".to_string());
    }
    match memberships.len() {
        0 => None,
        _ => Some(memberships[0].tenant_id.clone()),
    }
}
