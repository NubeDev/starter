//! `GET /auth/oauth/{provider}/login` — start the OAuth flow.
//!
//! Generates a fresh `state` + PKCE pair, stashes both (plus the
//! validated `return_to`) in the [`crate::OAuthStateStore`], and
//! `302`s the browser to the provider's authorize URL.
//!
//! `return_to` is filtered to relative paths — absolute URLs are
//! rejected with `400` so an attacker cannot turn the OAuth round
//! trip into an open redirect (CWE-601). The check is deliberately
//! conservative: anything that smells like a scheme (`http:`,
//! `https:`, `javascript:`, `//host/...`) is refused.

use std::sync::Arc;

use axum::http::header::LOCATION;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use chrono::Utc;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::state_store::OAuthFlowState;

use super::state::OAuthRoutesState;

/// Query parameters accepted by the start handler.
#[derive(Debug, Default, Deserialize)]
pub struct StartQuery {
    /// Where the browser should land after a successful callback.
    /// Must be a relative path (`/foo`, `/foo?bar=baz`). Absolute
    /// URLs are rejected. Defaults to
    /// [`OAuthRoutesState::default_return_to`].
    #[serde(default)]
    pub return_to: Option<String>,
}

/// Handler entry point. Returns either a `302` to the provider or a
/// `4xx` describing why the flow could not start.
pub async fn handler(
    state: Arc<OAuthRoutesState>,
    provider_id: String,
    query: StartQuery,
) -> Response {
    let provider = match state.providers.get(&provider_id) {
        Some(p) => p.clone(),
        // The path segment matched no enabled provider. SCOPE §"Routes"
        // calls this out: typos are a 404 here, not deep inside a
        // handler.
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    if let Some(rt) = query.return_to.as_deref() {
        if !is_safe_return_to(rt) {
            tracing::warn!(
                target: "starter_auth_oauth",
                provider = provider_id.as_str(),
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
        return_to: query.return_to.clone(),
        // Link-mode is Phase 3 (stage 8); the field is reserved here
        // so the wire shape is stable.
        link_mode_user_id: None,
        created_at: Utc::now(),
    };
    if let Err(e) = state.state_store.put(flow).await {
        tracing::warn!(target: "starter_auth_oauth", error = %e, "state store put failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let redirect_uri = format!(
        "{base}/auth/oauth/{provider}/callback",
        base = state.base_url.trim_end_matches('/'),
        provider = provider_id,
    );
    let url = provider.authorize_url(&state_value, &pkce_challenge, &redirect_uri);

    let mut resp = StatusCode::FOUND.into_response();
    if let Ok(v) = HeaderValue::from_str(&url) {
        resp.headers_mut().insert(LOCATION, v);
    }
    resp
}

/// `true` when `return_to` is a relative same-origin path the
/// callback can safely redirect to.
///
/// Conservative: rejects empty, anything starting with `//`, and
/// anything containing a colon in the first path segment (catches
/// `javascript:`, `data:`, scheme prefixes). The intentional shape is
/// "starts with `/` but not `//`."
pub(crate) fn is_safe_return_to(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if !s.starts_with('/') {
        return false;
    }
    if s.starts_with("//") {
        return false;
    }
    // Reject scheme-ish content in the first segment. A legitimate
    // path can contain a colon later (`/foo:bar`) without being a
    // scheme — only the leading segment matters for the open-redirect
    // attack model. Splitting at the first `?` or `#` keeps query +
    // fragment from being scanned.
    let first_seg = s.trim_start_matches('/');
    let first_seg = first_seg.split(['/', '?', '#']).next().unwrap_or(first_seg);
    if first_seg.contains(':') {
        return false;
    }
    true
}

pub(super) fn random_b64url(n: usize) -> String {
    let mut buf = vec![0u8; n];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

/// Build a fresh PKCE verifier + S256 challenge. Verifier is a
/// 32-byte URL-safe-no-pad string (~43 chars, well within the
/// RFC 7636 43–128 range); challenge is `base64url(sha256(verifier))`.
pub(super) fn new_pkce_pair() -> (String, String) {
    let verifier = random_b64url(32);
    let digest = Sha256::digest(verifier.as_bytes());
    let challenge = URL_SAFE_NO_PAD.encode(digest);
    (verifier, challenge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_to_relative_paths_pass() {
        assert!(is_safe_return_to("/"));
        assert!(is_safe_return_to("/foo"));
        assert!(is_safe_return_to("/foo/bar?x=1"));
        assert!(is_safe_return_to("/foo#bar"));
    }

    #[test]
    fn return_to_absolute_or_scheme_paths_fail() {
        assert!(!is_safe_return_to(""));
        assert!(!is_safe_return_to("foo"));
        assert!(!is_safe_return_to("//evil.example.com"));
        assert!(!is_safe_return_to("https://evil.example.com"));
        assert!(!is_safe_return_to("javascript:alert(1)"));
    }

    #[test]
    fn pkce_pair_round_trip_matches_s256_definition() {
        let (verifier, challenge) = new_pkce_pair();
        let expected = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        assert_eq!(challenge, expected);
    }
}
