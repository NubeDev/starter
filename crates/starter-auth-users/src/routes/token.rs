//! `POST /auth/token`. Body: `{ email, password, tenant_id? }`.
//! Returns a `sak_…` bearer the caller can present as
//! `Authorization: Bearer …`. The cookie-less counterpart of
//! [`super::login`] — mobile, native desktop, and CLI sign-in.
//!
//! Design + decision record:
//! [`rubix/docs/design/auth/token-issuance.md`](../../../../rubix/docs/design/auth/token-issuance.md).

use std::sync::Arc;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::password;
use crate::role::Role;
use crate::store::MembershipRecord;
use crate::token::{issue as issue_token, IssuedToken};

use super::login::PasswordNotSetResponse;
use super::state::AuthState;

/// Default bearer lifetime for `POST /auth/token`. Mobile clients
/// sit in pockets and there is no refresh token in v1, so a 30-day
/// TTL is the v1 trade-off (see design doc §TTL).
pub const TOKEN_DEFAULT_TTL_DAYS: i64 = 30;

/// Request body for `POST /auth/token`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
    /// User's email — same identifier as `POST /auth/login`.
    pub email: String,
    /// Plaintext password.
    pub password: String,
    /// Optional tenant binding. When omitted, the route resolves
    /// the tenant from the user's memberships (requires
    /// [`AuthState::with_tenants`]). See design doc §payload.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

/// Successful response body for `POST /auth/token`.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    /// The plaintext bearer (`sak_<id>.<secret>`). Shown once;
    /// the server stores only the argon2id hash of the secret.
    pub token: String,
    /// Absolute UTC expiry (RFC3339). Advisory in v1 — clients
    /// react to 401 rather than pre-emptively refreshing.
    pub expires_at: DateTime<Utc>,
    /// Always `"Bearer"`. Reserved for the future refresh-token
    /// flow.
    pub token_type: &'static str,
}

/// Body returned when the route cannot disambiguate the user's
/// tenant — multiple memberships exist and the client did not pass
/// `tenant_id`. The client re-POSTs with `tenant_id` set to one of
/// the entries below.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantRequiredResponse {
    /// Always `"tenant_required"`. Discriminator string.
    pub error: &'static str,
    /// One entry per membership row for the authenticated user.
    pub memberships: Vec<TenantMembershipEntry>,
}

/// One membership the user could pick on retry.
#[derive(Debug, Serialize, ToSchema)]
pub struct TenantMembershipEntry {
    /// Tenant id to echo back in `TokenRequest.tenant_id`.
    pub tenant_id: String,
    /// User's role within that tenant (`reader | writer | admin`).
    pub role: String,
}

impl From<&MembershipRecord> for TenantMembershipEntry {
    fn from(m: &MembershipRecord) -> Self {
        Self {
            tenant_id: m.tenant_id.clone(),
            role: m.role.clone(),
        }
    }
}

/// Body returned when `tenant_id` is required (no tenants store
/// wired) and the client omitted it.
#[derive(Debug, Serialize, ToSchema)]
pub struct MissingTenantIdResponse {
    /// Always `"missing_tenant_id"`.
    pub error: &'static str,
}

#[utoipa::path(
    post,
    path = "/auth/token",
    tag = "auth",
    operation_id = "issue_token",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Bearer issued", body = TokenResponse),
        (status = 400, description = "Account has no local password (parity with /auth/login), or tenants store unwired and tenant_id omitted",
            body = PasswordNotSetResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "tenant_id mismatch with memberships, or no tenant resolvable for a non-admin user"),
        (status = 409, description = "Multiple memberships exist; client must re-POST with explicit tenant_id",
            body = TenantRequiredResponse),
    ),
)]
pub(crate) async fn handler(
    state: Arc<AuthState>,
    Json(body): Json<TokenRequest>,
) -> Response {
    // --- 1. Resolve user + verify password (parity with login) ---
    let user = match state.users.find_by_email(&body.email).await {
        Ok(Some(u)) => u,
        Ok(None) => return StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "token store lookup failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let password_hash = match &user.password_hash {
        Some(h) => h.as_str(),
        None => {
            // OAuth-only account. Return the SAME envelope login
            // emits so clients pattern-match on `error:
            // "password_not_set"` for either route.
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
            tracing::warn!(target: "starter_auth_users", error = %e, "token password verify failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }

    // --- 2. Resolve tenant binding -----------------------------------
    let tenant_id = match resolve_tenant(state.as_ref(), &user.id, &user.role, body.tenant_id).await
    {
        Ok(t) => t,
        Err(resp) => return resp,
    };

    // --- 3. Mint the token -------------------------------------------
    let expires_at = Utc::now() + Duration::days(TOKEN_DEFAULT_TTL_DAYS);
    let issued: IssuedToken = match issue_token(
        state.tokens.as_ref(),
        &user.id,
        &[],
        &tenant_id,
        Some(expires_at),
    )
    .await
    {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "token issue failed");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    (
        StatusCode::OK,
        Json(TokenResponse {
            token: issued.plaintext,
            expires_at,
            token_type: "Bearer",
        }),
    )
        .into_response()
}

/// Pick the tenant the new bearer will be bound to. Either the
/// caller passes one (validated against memberships), or we resolve
/// it from [`crate::store::TenantStore::memberships_for_user`].
///
/// Errors are pre-rendered HTTP `Response`s so the call site can
/// `return` them directly.
async fn resolve_tenant(
    state: &AuthState,
    user_id: &str,
    user_role: &Role,
    requested: Option<String>,
) -> Result<String, Response> {
    // Without a tenants store the only way forward is an
    // explicit tenant_id. Admins still need to opt in — they
    // could pass `"*"` to bypass cross-tenant filters per the
    // token::issue contract.
    let Some(tenants) = state.tenants.clone() else {
        return match requested {
            Some(t) => Ok(t),
            None => Err((
                StatusCode::BAD_REQUEST,
                Json(MissingTenantIdResponse {
                    error: "missing_tenant_id",
                }),
            )
                .into_response()),
        };
    };

    let memberships = match tenants.memberships_for_user(user_id).await {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(target: "starter_auth_users", error = %e, "memberships lookup failed");
            return Err(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    if let Some(requested) = requested {
        // Super-admin sentinel bypass — only granted to global
        // Admins. Matches the `SUPER_ADMIN_TENANT` contract in
        // `token::issue`.
        if requested == "*" {
            return if matches!(user_role, Role::Admin) {
                Ok(requested)
            } else {
                Err(StatusCode::FORBIDDEN.into_response())
            };
        }
        return if memberships.iter().any(|m| m.tenant_id == requested) {
            Ok(requested)
        } else {
            Err(StatusCode::FORBIDDEN.into_response())
        };
    }

    match memberships.len() {
        1 => Ok(memberships[0].tenant_id.clone()),
        0 => {
            // No memberships, but a global Admin can still mint
            // against the super-admin sentinel — matches the
            // cookie-session admin paths.
            if matches!(user_role, Role::Admin) {
                Ok("*".to_string())
            } else {
                Err(StatusCode::FORBIDDEN.into_response())
            }
        }
        _ => Err((
            StatusCode::CONFLICT,
            Json(TenantRequiredResponse {
                error: "tenant_required",
                memberships: memberships.iter().map(Into::into).collect(),
            }),
        )
            .into_response()),
    }
}
