//! `GET /auth/oauth/{provider}/callback` — the only state-changing
//! `GET` we ship (Hard rule R9). CSRF is provided by the OAuth
//! `state` parameter: random, single-use, bound to the user's
//! browser via the state-store entry.
//!
//! The handler walks the seven-branch decision tree from SCOPE
//! §"Flow (callback handler)":
//!
//! 1. `find` hit + no link-mode  → **sign-in** to the linked user.
//! 2. `find` hit + link-mode = u → **link-hit**: identity already
//!    belongs to `u` (idempotent sign-in) or to a different user
//!    (`HTTP 409 already_linked_to_other`).
//! 3. `find` miss + link-mode = u → **link-miss**: insert a new
//!    identity row for `u` and mint a session.
//! 4. `find` miss + no link-mode + verified email matches an
//!    existing user → **verified-match link**: insert identity,
//!    mint session for that user.
//! 5. `find` miss + no link-mode + email matches but `verified=false`
//!    → **unverified collision**: `HTTP 409
//!    email_already_registered` (Hard rule R3).
//! 6. `find` miss + no link-mode + no email match + signup_enabled
//!    → **signup**: create user + identity, mint session.
//! 7. `find` miss + no link-mode + no email match + signup disabled
//!    → `HTTP 403 signup_disabled`.
//!
//! Error mapping is deliberate (SCOPE):
//!
//! - The user-facing response is `sign_in_failed` plus a correlation
//!   id; the underlying reason is in the `tracing` event keyed by
//!   the same correlation id. An attacker probing the callback
//!   cannot tell a bad `state` from a bad `code` from a transport
//!   blip; the operator looking at logs always can.

use std::sync::Arc;

use axum::http::header::LOCATION;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use crate::identity_store::{IdentityStoreError, OAuthIdentity};
use crate::provider::ProviderIdentity;
use crate::session_bridge::mint_session_headers;
use crate::state_store::OAuthFlowState;

use super::state::OAuthRoutesState;

/// Query parameters the provider appends to its callback redirect.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    /// Single-use authorization code we exchange for a token.
    /// `None` when the provider rejected the consent — the
    /// `error` field carries the reason in that case.
    #[serde(default)]
    pub code: Option<String>,
    /// CSRF token we minted at start; must match a live state
    /// store entry.
    #[serde(default)]
    pub state: Option<String>,
    /// Provider-side error code (`access_denied` etc.). When
    /// populated, `code` is `None` and we never touch the state
    /// store; the user just lands on `return_to` with a failure.
    #[serde(default)]
    pub error: Option<String>,
    /// Human-readable provider error description.
    #[serde(default)]
    pub error_description: Option<String>,
}

/// Outcome that drives the final HTTP response. Each branch maps
/// 1:1 onto the seven cases listed at the top of this file.
enum Outcome {
    /// Sign the user in. Carries the user id whose session we mint.
    SignIn(String),
    /// `(StatusCode, error_code)` — user-facing failure.
    Fail(StatusCode, &'static str),
}

/// Entry point. Splits the work into resolution (state-store + provider
/// IO + DB branching) and rendering (cookie-mint + redirect) so the
/// render layer never holds the access token in scope.
pub async fn handler(
    state: Arc<OAuthRoutesState>,
    provider_id: String,
    query: CallbackQuery,
) -> Response {
    let correlation_id = uuid::Uuid::new_v4().to_string();

    let provider = match state.providers.get(&provider_id) {
        Some(p) => p.clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if let Some(err) = query.error.as_deref() {
        tracing::warn!(
            target: "starter_auth_oauth",
            correlation_id = correlation_id.as_str(),
            provider = provider_id.as_str(),
            provider_error = err,
            provider_error_description = query.error_description.as_deref().unwrap_or(""),
            "provider returned an error response",
        );
        return sign_in_failed(StatusCode::BAD_REQUEST, "sign_in_failed", &correlation_id);
    }

    let (code, state_value) = match (query.code.as_deref(), query.state.as_deref()) {
        (Some(c), Some(s)) if !c.is_empty() && !s.is_empty() => (c.to_string(), s.to_string()),
        _ => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id = correlation_id.as_str(),
                provider = provider_id.as_str(),
                "callback missing code or state",
            );
            return sign_in_failed(StatusCode::BAD_REQUEST, "sign_in_failed", &correlation_id);
        }
    };

    // Atomic take: a forged or replayed callback finds nothing and
    // dies here without doing any provider IO (Hard rule R5).
    let flow = match state.state_store.take(&state_value).await {
        Ok(Some(f)) => f,
        Ok(None) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id = correlation_id.as_str(),
                provider = provider_id.as_str(),
                "state token not found, expired, or already consumed",
            );
            return sign_in_failed(StatusCode::BAD_REQUEST, "sign_in_failed", &correlation_id);
        }
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id = correlation_id.as_str(),
                error = %e,
                "state store take failed",
            );
            return sign_in_failed(StatusCode::INTERNAL_SERVER_ERROR, "sign_in_failed", &correlation_id);
        }
    };

    // The provider id baked into the flow must match the path
    // segment; mismatch is a tampered URL and we refuse it without
    // hitting the network.
    if flow.provider != provider_id {
        tracing::warn!(
            target: "starter_auth_oauth",
            correlation_id = correlation_id.as_str(),
            flow_provider = flow.provider.as_str(),
            path_provider = provider_id.as_str(),
            "callback path/provider mismatch",
        );
        return sign_in_failed(StatusCode::BAD_REQUEST, "sign_in_failed", &correlation_id);
    }

    let redirect_uri = format!(
        "{base}/auth/oauth/{provider}/callback",
        base = state.base_url.trim_end_matches('/'),
        provider = provider_id,
    );

    // Provider round trip. The access token never leaves
    // `fetch_identity` (Hard rule R2).
    let identity = match provider
        .fetch_identity(&code, &flow.pkce_verifier, &redirect_uri)
        .await
    {
        Ok(i) => i,
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id = correlation_id.as_str(),
                error = %e,
                "fetch_identity failed",
            );
            return sign_in_failed(StatusCode::BAD_REQUEST, "sign_in_failed", &correlation_id);
        }
    };

    let outcome = resolve(&state, &provider_id, &flow, &identity, &correlation_id).await;
    render(state, outcome, &flow, &correlation_id).await
}

/// Resolve the seven-branch tree into an [`Outcome`]. Kept separate
/// from the HTTP rendering so the unit tests can drive the branching
/// directly.
async fn resolve(
    state: &OAuthRoutesState,
    provider_id: &str,
    flow: &OAuthFlowState,
    identity: &ProviderIdentity,
    correlation_id: &str,
) -> Outcome {
    let found = match state
        .identity_store
        .find(provider_id, &identity.provider_sub)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id,
                error = %e,
                "identity_store.find failed",
            );
            return Outcome::Fail(StatusCode::INTERNAL_SERVER_ERROR, "sign_in_failed");
        }
    };

    match (found, flow.link_mode_user_id.as_deref()) {
        // Branch 1: sign-in hit.
        (Some(row), None) => {
            tracing::info!(
                target: "starter_auth_oauth",
                correlation_id,
                provider = provider_id,
                user_id = row.user_id.as_str(),
                action = "signin",
                "oauth sign-in resolved",
            );
            Outcome::SignIn(row.user_id)
        }
        // Branch 2: link hit. Identity already exists; if it points
        // at the same user that's a no-op sign-in, otherwise the
        // identity belongs to someone else and we refuse.
        (Some(row), Some(link_user)) => {
            if row.user_id == link_user {
                Outcome::SignIn(row.user_id)
            } else {
                tracing::warn!(
                    target: "starter_auth_oauth",
                    correlation_id,
                    provider = provider_id,
                    "identity already linked to a different user",
                );
                Outcome::Fail(StatusCode::CONFLICT, "already_linked_to_other_user")
            }
        }
        // Branch 3: link miss — insert under the logged-in user.
        (None, Some(link_user)) => {
            let row = OAuthIdentity {
                provider: provider_id.to_string(),
                provider_sub: identity.provider_sub.clone(),
                user_id: link_user.to_string(),
                email: Some(identity.email.clone()),
                display_name: identity.display_name.clone(),
                linked_at: Utc::now(),
            };
            if let Err(e) = state.identity_store.insert(&row).await {
                return identity_insert_failure(e, correlation_id);
            }
            tracing::info!(
                target: "starter_auth_oauth",
                correlation_id,
                provider = provider_id,
                user_id = link_user,
                action = "link",
                "oauth identity linked",
            );
            Outcome::SignIn(link_user.to_string())
        }
        // Branches 4–7: no identity row, no link-mode → email-based
        // matching plus optional signup.
        (None, None) => match state.user_store.find_by_email(&identity.email).await {
            // Branch 4 / 5: an existing user has this email.
            Ok(Some(existing)) => {
                if !identity.email_verified {
                    // Branch 5: unverified collision (Hard rule R3).
                    tracing::warn!(
                        target: "starter_auth_oauth",
                        correlation_id,
                        provider = provider_id,
                        user_id = existing.id.as_str(),
                        "refusing to link unverified provider email to existing user",
                    );
                    return Outcome::Fail(StatusCode::CONFLICT, "email_already_registered");
                }
                // Branch 4: verified-match auto-link.
                let row = OAuthIdentity {
                    provider: provider_id.to_string(),
                    provider_sub: identity.provider_sub.clone(),
                    user_id: existing.id.clone(),
                    email: Some(identity.email.clone()),
                    display_name: identity.display_name.clone(),
                    linked_at: Utc::now(),
                };
                if let Err(e) = state.identity_store.insert(&row).await {
                    return identity_insert_failure(e, correlation_id);
                }
                tracing::info!(
                    target: "starter_auth_oauth",
                    correlation_id,
                    provider = provider_id,
                    user_id = existing.id.as_str(),
                    action = "link",
                    "oauth identity auto-linked on verified email match",
                );
                Outcome::SignIn(existing.id)
            }
            // Branches 6 / 7: no existing user → signup gate.
            Ok(None) => {
                if !identity.email_verified {
                    // R3 again: signups also need a verified email.
                    // GitHub's fetch_identity already enforces this
                    // upstream; the check is a belt-and-suspenders
                    // guard for FakeProvider tests and future
                    // providers.
                    tracing::warn!(
                        target: "starter_auth_oauth",
                        correlation_id,
                        provider = provider_id,
                        "refusing signup on unverified email",
                    );
                    return Outcome::Fail(StatusCode::CONFLICT, "email_not_verified");
                }
                if !state.signup_enabled {
                    // Branch 7.
                    tracing::warn!(
                        target: "starter_auth_oauth",
                        correlation_id,
                        provider = provider_id,
                        "signup disabled; first-time callback refused",
                    );
                    return Outcome::Fail(StatusCode::FORBIDDEN, "signup_disabled");
                }
                // Branch 6: signup. `password_hash = None` is the
                // OAuth-only marker; `POST /auth/login` will surface
                // `password_not_set` if the user later tries the
                // password path.
                //
                // Role assignment: the verified email's domain is
                // checked against this provider's role-domain map.
                // On hit the matched role wins; on miss the user
                // falls back to `OAUTH_SIGNUP_DEFAULT_ROLE`. We only
                // reach this branch with `email_verified == true`
                // (the guard above), so an unverified-email-controlled
                // domain cannot inject a privileged role.
                let role = resolve_signup_role(state, provider_id, &identity.email);
                let new_user_id = uuid::Uuid::new_v4().to_string();
                if let Err(e) = state
                    .user_store
                    .create(&new_user_id, &identity.email, None, role)
                    .await
                {
                    tracing::warn!(
                        target: "starter_auth_oauth",
                        correlation_id,
                        error = %e,
                        "user_store.create failed during signup",
                    );
                    return Outcome::Fail(StatusCode::INTERNAL_SERVER_ERROR, "sign_in_failed");
                }
                let row = OAuthIdentity {
                    provider: provider_id.to_string(),
                    provider_sub: identity.provider_sub.clone(),
                    user_id: new_user_id.clone(),
                    email: Some(identity.email.clone()),
                    display_name: identity.display_name.clone(),
                    linked_at: Utc::now(),
                };
                if let Err(e) = state.identity_store.insert(&row).await {
                    return identity_insert_failure(e, correlation_id);
                }
                tracing::info!(
                    target: "starter_auth_oauth",
                    correlation_id,
                    provider = provider_id,
                    user_id = new_user_id.as_str(),
                    action = "signup",
                    role = ?role,
                    "oauth signup created new local user",
                );
                Outcome::SignIn(new_user_id)
            }
            Err(e) => {
                tracing::warn!(
                    target: "starter_auth_oauth",
                    correlation_id,
                    error = %e,
                    "user_store.find_by_email failed",
                );
                Outcome::Fail(StatusCode::INTERNAL_SERVER_ERROR, "sign_in_failed")
            }
        },
    }
}

/// Look up the verified email's domain in the provider's
/// `role_domain_map`. Fall back to `signup_default_role` on any
/// miss: no entry for the provider, no entry for this domain, or an
/// email shape we can't split.
fn resolve_signup_role(
    state: &OAuthRoutesState,
    provider_id: &str,
    email: &str,
) -> starter_auth_users::Role {
    let map = match state.role_domain_maps.get(provider_id) {
        Some(m) if !m.is_empty() => m,
        _ => return state.signup_default_role,
    };
    let Some((_, domain)) = email.rsplit_once('@') else {
        return state.signup_default_role;
    };
    let domain = domain.trim().to_ascii_lowercase();
    if domain.is_empty() {
        return state.signup_default_role;
    }
    map.get(&domain)
        .copied()
        .unwrap_or(state.signup_default_role)
}

fn identity_insert_failure(e: IdentityStoreError, correlation_id: &str) -> Outcome {
    match e {
        IdentityStoreError::Conflict => {
            // Two callbacks racing for the same (provider, sub).
            // Surfacing the bare conflict is fine — the user just
            // retries and the second attempt sees the linked row.
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id,
                "identity insert lost a race on composite key",
            );
            Outcome::Fail(StatusCode::CONFLICT, "already_linked")
        }
        e => {
            tracing::warn!(
                target: "starter_auth_oauth",
                correlation_id,
                error = %e,
                "identity_store.insert failed",
            );
            Outcome::Fail(StatusCode::INTERNAL_SERVER_ERROR, "sign_in_failed")
        }
    }
}

async fn render(
    state: Arc<OAuthRoutesState>,
    outcome: Outcome,
    flow: &OAuthFlowState,
    correlation_id: &str,
) -> Response {
    match outcome {
        Outcome::Fail(code, err) => sign_in_failed(code, err, correlation_id),
        Outcome::SignIn(user_id) => {
            let headers = match mint_session_headers(state.session_store.clone(), &user_id).await {
                Ok(h) => h,
                Err(e) => {
                    tracing::warn!(
                        target: "starter_auth_oauth",
                        correlation_id,
                        error = %e,
                        "session mint failed",
                    );
                    return sign_in_failed(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "sign_in_failed",
                        correlation_id,
                    );
                }
            };

            let target = flow
                .return_to
                .clone()
                .unwrap_or_else(|| state.default_return_to.clone());

            let mut resp = StatusCode::FOUND.into_response();
            *resp.headers_mut() = headers;
            if let Ok(v) = HeaderValue::from_str(&target) {
                resp.headers_mut().insert(LOCATION, v);
            }
            resp
        }
    }
}

fn sign_in_failed(status: StatusCode, code: &'static str, correlation_id: &str) -> Response {
    (
        status,
        Json(json!({
            "error": code,
            "correlation_id": correlation_id,
        })),
    )
        .into_response()
}
