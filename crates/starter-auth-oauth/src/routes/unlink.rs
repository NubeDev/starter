//! `DELETE /auth/oauth/{provider}` — remove every identity the
//! current user has under `{provider}`.
//!
//! Per Hard rule R4 the handler refuses with `409` when the deletion
//! would leave the user with no way to sign in — that is, the user
//! has no local password (`password_hash IS NULL`) **and** the
//! provider being unlinked is their only linked identity. The
//! identity rows are left untouched in that case so a subsequent
//! `POST /link` can still proceed against the same `(provider, sub)`
//! key; refusing the delete is the *only* observable consequence.
//!
//! Returns:
//! - `204 No Content` on success (the identity row was deleted, or
//!   was already absent — idempotent).
//! - `401 Unauthorized` when no session cookie.
//! - `403 Forbidden` when the CSRF double-submit fails.
//! - `404 Not Found` when the `{provider}` segment names no enabled
//!   provider.
//! - `409 Conflict` with `{ "error": "last_sign_in_method" }` when
//!   the deletion would orphan the user.
//! - `500 Internal Server Error` on backing-store failure.

use std::sync::Arc;

use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use super::session_guard::{require_session_csrf, GuardOutcome};
use super::state::OAuthRoutesState;

/// Handler entry point.
pub async fn handler(
    state: Arc<OAuthRoutesState>,
    provider_id: String,
    headers: HeaderMap,
) -> Response {
    let user_id = match require_session_csrf(
        state.session_store.as_ref(),
        state.user_store.as_ref(),
        &headers,
        true,
    )
    .await
    {
        GuardOutcome::Allow(uid) => uid,
        GuardOutcome::Deny(status) => return status.into_response(),
    };

    if !state.providers.contains_key(&provider_id) {
        return StatusCode::NOT_FOUND.into_response();
    }

    // We need the full list to count *other* identities and to know
    // which `(provider, provider_sub)` rows belong to this provider.
    let rows = match state.identity_store.list_for_user(&user_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                user_id = user_id.as_str(),
                error = %e,
                "identity_store.list_for_user failed during unlink",
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let (to_remove, others): (Vec<_>, Vec<_>) =
        rows.into_iter().partition(|r| r.provider == provider_id);

    if to_remove.is_empty() {
        // Idempotent: nothing to do. We still emit the audit event so
        // an operator can correlate the (otherwise silent) request.
        tracing::info!(
            target: "starter_auth_oauth",
            provider = provider_id.as_str(),
            user_id = user_id.as_str(),
            action = "unlink_noop",
            "no identity row to unlink",
        );
        return StatusCode::NO_CONTENT.into_response();
    }

    // The R4 refusal: this provider is the *only* identity AND there
    // is no local password to fall back on. Fetching the user record
    // is the cheap way to read `password_hash IS NULL` without adding
    // a dedicated count query to `UserStore`.
    if others.is_empty() {
        let user = match state.user_store.find_by_id(&user_id).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                // The session resolved a user that no longer exists.
                // Treat as unauthorised so the SPA prompts a fresh login.
                return StatusCode::UNAUTHORIZED.into_response();
            }
            Err(e) => {
                tracing::warn!(
                    target: "starter_auth_oauth",
                    user_id = user_id.as_str(),
                    error = %e,
                    "user_store.find_by_id failed during unlink",
                );
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        if user.password_hash.is_none() {
            tracing::warn!(
                target: "starter_auth_oauth",
                provider = provider_id.as_str(),
                user_id = user_id.as_str(),
                action = "unlink_refused",
                "refused to remove last sign-in method",
            );
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "last_sign_in_method",
                    "message": "cannot remove the only remaining sign-in method; \
                                set a password first or link another provider",
                })),
            )
                .into_response();
        }
    }

    // Composite key per row; `IdentityStore::delete` is itself
    // idempotent so a race where two `DELETE` calls land
    // simultaneously is harmless.
    for row in &to_remove {
        if let Err(e) = state
            .identity_store
            .delete(&row.provider, &row.provider_sub)
            .await
        {
            tracing::warn!(
                target: "starter_auth_oauth",
                provider = provider_id.as_str(),
                user_id = user_id.as_str(),
                error = %e,
                "identity_store.delete failed during unlink",
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        tracing::info!(
            target: "starter_auth_oauth",
            provider = provider_id.as_str(),
            user_id = user_id.as_str(),
            action = "unlink",
            "oauth identity unlinked",
        );
    }

    StatusCode::NO_CONTENT.into_response()
}
