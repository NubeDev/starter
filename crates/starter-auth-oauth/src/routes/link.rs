//! `POST /auth/oauth/{provider}/link` — start an OAuth flow on
//! behalf of the currently signed-in user.
//!
//! This is the entry point Hard rule R4 names: logged-in linking is
//! a separate, explicit flow. Mechanically it mirrors
//! [`super::start::handler`] — generate fresh `state` + PKCE, stash
//! an [`OAuthFlowState`] in the [`crate::OAuthStateStore`], and tell
//! the browser where to go — with one critical difference: the flow
//! row carries `link_mode_user_id = Some(current_user)`, which is
//! the marker [`super::callback::handler`] reads to route the same
//! provider round-trip into the *link* branch of its seven-branch
//! decision tree (Branch 2 / 3) instead of the sign-in branch
//! (Branch 1 / 4 / 6).
//!
//! Unlike `start.rs`, this is a `POST` with session + CSRF (R9: the
//! callback `GET` is the only state-changing `GET`; everything else
//! uses the standard double-submit cookie). The handler returns a
//! JSON body carrying the provider's authorize URL rather than a
//! `302` — the caller is JavaScript with a CSRF token in hand, not
//! a browser navigation, and serving a redirect from a `POST` would
//! force the SPA to follow it through `fetch` semantics.

use std::sync::Arc;

use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::state_store::OAuthFlowState;

use super::session_guard::{require_session_csrf, GuardOutcome};
use super::start::{is_safe_return_to, new_pkce_pair, random_b64url};
use super::state::OAuthRoutesState;

/// JSON body accepted by the link handler. Both fields are optional;
/// only `return_to` is consumer-supplied.
#[derive(Debug, Default, Deserialize)]
pub struct LinkRequest {
    /// Where the browser lands after the callback succeeds. Same
    /// shape and same open-redirect filter as
    /// [`super::start::StartQuery::return_to`].
    #[serde(default)]
    pub return_to: Option<String>,
}

/// JSON body returned on a successful link start. The caller (an
/// SPA) opens `authorize_url` in the same browser window so the
/// provider redirect lands back at `GET /auth/oauth/{provider}/callback`
/// with the link-mode marker stashed in the state-store entry.
#[derive(Debug, Serialize)]
pub struct LinkResponse {
    /// Fully-qualified URL the SPA should navigate to.
    pub authorize_url: String,
}

/// Handler entry point. Returns `200 { authorize_url }` on success,
/// `401` on no session, `403` on missing / mismatched CSRF, `404` on
/// unknown provider, `400` on a malformed `return_to`.
pub async fn handler(
    state: Arc<OAuthRoutesState>,
    provider_id: String,
    headers: HeaderMap,
    body: Option<Json<LinkRequest>>,
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

    let provider = match state.providers.get(&provider_id) {
        Some(p) => p.clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let req = body.map(|Json(b)| b).unwrap_or_default();
    if let Some(rt) = req.return_to.as_deref() {
        if !is_safe_return_to(rt) {
            tracing::warn!(
                target: "starter_auth_oauth",
                provider = provider_id.as_str(),
                user_id = user_id.as_str(),
                "rejecting absolute return_to to avoid open-redirect",
            );
            return (StatusCode::BAD_REQUEST, "invalid return_to").into_response();
        }
    }

    let state_value = random_b64url(32);
    let (pkce_verifier, pkce_challenge) = new_pkce_pair();

    let flow = OAuthFlowState {
        provider: provider_id.clone(),
        state: state_value.clone(),
        pkce_verifier,
        return_to: req.return_to,
        // The marker that turns this into a link flow. The callback
        // handler reads this field to choose the link branch over
        // the sign-in branch (Hard rule R4).
        link_mode_user_id: Some(user_id.clone()),
        created_at: Utc::now(),
    };
    if let Err(e) = state.state_store.put(flow).await {
        tracing::warn!(
            target: "starter_auth_oauth",
            error = %e,
            provider = provider_id.as_str(),
            "state store put failed during link start",
        );
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let redirect_uri = format!(
        "{base}/auth/oauth/{provider}/callback",
        base = state.base_url.trim_end_matches('/'),
        provider = provider_id,
    );
    let authorize_url = provider.authorize_url(&state_value, &pkce_challenge, &redirect_uri);

    tracing::info!(
        target: "starter_auth_oauth",
        provider = provider_id.as_str(),
        user_id = user_id.as_str(),
        action = "link_start",
        "oauth link flow started",
    );

    (StatusCode::OK, Json(LinkResponse { authorize_url })).into_response()
}
