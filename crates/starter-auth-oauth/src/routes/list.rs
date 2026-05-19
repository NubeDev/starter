//! `GET /auth/oauth/identities` — list the third-party identities
//! linked to the currently signed-in user.
//!
//! Read-only, so CSRF is **not** required (the standard
//! double-submit pattern protects state-changing requests). A valid
//! session is enough.
//!
//! The shape is intentionally narrow: `(provider, email,
//! display_name, last_login_at)`. The composite primary key
//! (`provider_sub`) is deliberately omitted — the SPA only needs
//! enough to render a "linked accounts" list and to wire up an
//! "unlink" button (which targets `DELETE /auth/oauth/{provider}`),
//! and `provider_sub` would leak a stable provider-side identifier
//! into the browser for no benefit.
//!
//! `last_login_at` in v0.1 mirrors the row's `linked_at` timestamp —
//! we do not yet touch the row on a sign-in hit, so "most recent
//! successful sign-in via this identity" collapses to "when the
//! identity was first linked." A later stage can split the two by
//! adding an `IdentityStore::touch_last_login` method; the wire
//! shape here is forward-compatible.

use std::sync::Arc;

use axum::http::header::HeaderMap;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;

use super::session_guard::{require_session_csrf, GuardOutcome};
use super::state::OAuthRoutesState;

/// One element of the response — the public projection of an
/// [`crate::OAuthIdentity`] row.
#[derive(Debug, Serialize)]
pub struct IdentitySummary {
    /// Provider id (`"github"`, `"google"`).
    pub provider: String,
    /// Email the provider returned at link / last sign-in.
    pub email: Option<String>,
    /// Display name the provider returned at link / last sign-in.
    pub display_name: Option<String>,
    /// Timestamp of the most recent successful sign-in via this
    /// identity. See the module doc for the v0.1 equivalence with
    /// `linked_at`.
    pub last_login_at: DateTime<Utc>,
}

/// Top-level response. A JSON object so the wire shape stays
/// extensible — adding `count`, paging metadata, etc. is additive.
#[derive(Debug, Serialize)]
pub struct IdentitiesResponse {
    /// The user's linked identities, ordered by `linked_at` ascending
    /// (the longest-held identity first, matching the contract
    /// `IdentityStore::list_for_user` already documents).
    pub identities: Vec<IdentitySummary>,
}

/// Handler entry point.
pub async fn handler(state: Arc<OAuthRoutesState>, headers: HeaderMap) -> Response {
    let user_id = match require_session_csrf(
        state.session_store.as_ref(),
        state.user_store.as_ref(),
        &headers,
        // Read-only endpoint: a session is enough, CSRF is not
        // required (the OAuth `state` token is the CSRF for the
        // callback `GET`; the standard double-submit cookie guards
        // state-changing requests like `link` and `unlink`).
        false,
    )
    .await
    {
        GuardOutcome::Allow(uid) => uid,
        GuardOutcome::Deny(status) => return status.into_response(),
    };

    let rows = match state.identity_store.list_for_user(&user_id).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                target: "starter_auth_oauth",
                user_id = user_id.as_str(),
                error = %e,
                "identity_store.list_for_user failed",
            );
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let identities = rows
        .into_iter()
        .map(|r| IdentitySummary {
            provider: r.provider,
            email: r.email,
            display_name: r.display_name,
            last_login_at: r.linked_at,
        })
        .collect();

    (StatusCode::OK, Json(IdentitiesResponse { identities })).into_response()
}
