//! Phase 3 (stage 8) coverage — `POST /link`, `DELETE /unlink`, and
//! `GET /identities`.
//!
//! The headline test is **Unlinking-last-sign-in-method-is-refused**:
//! a user created via OAuth-only (no local password) with exactly one
//! linked identity must not be able to delete that identity. The
//! refusal is the only thing standing between R4 ("unlinking is
//! refused if it would leave the user with no way to sign in") and
//! a self-service account lockout.
//!
//! The other tests cover the link-mode marker round-trip and the
//! identity-listing wire shape, plus the negative-case
//! permission ladder (no session → 401; no CSRF → 403).

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::header::{HeaderMap, HeaderValue, COOKIE};
use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::routes::{
    link_handler, list_handler, unlink_handler, LinkRequest, OAuthRoutesState,
};
use starter_auth_oauth::session_bridge::{mint_session_headers, CSRF_COOKIE};
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{OAuthIdentity, ProviderIdentity};
use starter_auth_users::session::SESSION_COOKIE;
use starter_auth_users::Role;

const PROVIDER: &str = "github";

fn ident_default() -> ProviderIdentity {
    ProviderIdentity {
        provider_sub: "sub-1".into(),
        email: "ada@example.com".into(),
        email_verified: true,
        display_name: Some("Ada".into()),
    }
}

async fn build() -> (Arc<FakeProvider>, MemoryEverything) {
    let provider = FakeProvider::new(PROVIDER);
    provider.set_identity(ident_default());
    let me = MemoryEverything::new(vec![provider.clone()]).await;
    (provider, me)
}

/// Create a user + session + CSRF and return a `HeaderMap` ready to
/// pass to a handler. `password_hash` controls whether the user has a
/// local password (relevant to the R4 refusal logic).
async fn seed_session(
    state: &OAuthRoutesState,
    user_id: &str,
    email: &str,
    password_hash: Option<&str>,
) -> HeaderMap {
    state
        .user_store
        .create(user_id, email, password_hash, Role::Reader)
        .await
        .unwrap();
    let mint = mint_session_headers(state.session_store.clone(), user_id)
        .await
        .expect("mint session");

    // Extract cookie values from the Set-Cookie headers minted by
    // `session_bridge`; the test then re-emits them as a `Cookie`
    // header on the next request.
    let mut session_value = None;
    let mut csrf_value = None;
    for v in mint.get_all(axum::http::header::SET_COOKIE) {
        let s = v.to_str().unwrap();
        let first = s.split(';').next().unwrap();
        if let Some(rest) = first.strip_prefix(&format!("{SESSION_COOKIE}=")) {
            session_value = Some(rest.to_string());
        }
        if let Some(rest) = first.strip_prefix(&format!("{CSRF_COOKIE}=")) {
            csrf_value = Some(rest.to_string());
        }
    }
    let session_value = session_value.expect("session cookie minted");
    let csrf_value = csrf_value.expect("csrf cookie minted");

    let mut headers = HeaderMap::new();
    headers.insert(
        COOKIE,
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}={session_value}; {CSRF_COOKIE}={csrf_value}"
        ))
        .unwrap(),
    );
    headers.insert(
        "x-csrf-token",
        HeaderValue::from_str(&csrf_value).unwrap(),
    );
    headers
}

async fn body_text(resp: axum::response::Response) -> (StatusCode, String) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn unlinking_last_sign_in_method_is_refused() {
    // The smoke test the source SCOPE names by hand. An OAuth-only
    // user (no `password_hash`) with one linked identity must hit a
    // 409 — and the identity row must still be there after the
    // refusal so the user can keep signing in.
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    state
        .identity_store
        .insert(&OAuthIdentity {
            provider: PROVIDER.into(),
            provider_sub: "sub-1".into(),
            user_id: "u-1".into(),
            email: Some("ada@example.com".into()),
            display_name: Some("Ada".into()),
            linked_at: Utc::now(),
        })
        .await
        .unwrap();

    let resp = unlink_handler(state.clone(), PROVIDER.into(), headers).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(
        body.contains("last_sign_in_method"),
        "body was {body:?}"
    );

    // R4 requires the row to remain so the user is not locked out.
    let row = state
        .identity_store
        .find(PROVIDER, "sub-1")
        .await
        .unwrap();
    assert!(row.is_some(), "identity row must remain after refusal");
}

#[tokio::test]
async fn unlinking_succeeds_when_user_has_password_fallback() {
    // Same setup but the user has a local password — the unlink
    // proceeds because the user can still sign in via `/auth/login`.
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", Some("$argon2id$dummy")).await;
    state
        .identity_store
        .insert(&OAuthIdentity {
            provider: PROVIDER.into(),
            provider_sub: "sub-1".into(),
            user_id: "u-1".into(),
            email: Some("ada@example.com".into()),
            display_name: None,
            linked_at: Utc::now(),
        })
        .await
        .unwrap();

    let resp = unlink_handler(state.clone(), PROVIDER.into(), headers).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(state
        .identity_store
        .find(PROVIDER, "sub-1")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn unlinking_succeeds_when_other_identity_remains() {
    // No local password, but a second provider (google) is linked —
    // dropping the github identity still leaves a sign-in path.
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    state
        .identity_store
        .insert(&OAuthIdentity {
            provider: PROVIDER.into(),
            provider_sub: "sub-1".into(),
            user_id: "u-1".into(),
            email: Some("ada@example.com".into()),
            display_name: None,
            linked_at: Utc::now(),
        })
        .await
        .unwrap();
    state
        .identity_store
        .insert(&OAuthIdentity {
            provider: "google".into(),
            provider_sub: "google-sub-1".into(),
            user_id: "u-1".into(),
            email: Some("ada@example.com".into()),
            display_name: None,
            linked_at: Utc::now(),
        })
        .await
        .unwrap();

    let resp = unlink_handler(state.clone(), PROVIDER.into(), headers).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let remaining = state
        .identity_store
        .list_for_user("u-1")
        .await
        .unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].provider, "google");
}

#[tokio::test]
async fn unlink_without_session_is_unauthorized() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let resp = unlink_handler(state, PROVIDER.into(), HeaderMap::new()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unlink_without_csrf_is_forbidden() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let mut headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    headers.remove("x-csrf-token");

    let resp = unlink_handler(state, PROVIDER.into(), headers).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn link_handler_stashes_link_mode_user_id() {
    // The link-mode marker is the load-bearing field for R4 routing.
    // We assert by pulling the freshly-stashed flow back out of the
    // state store and checking `link_mode_user_id`.
    let (provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-link", "link@example.com", None).await;

    let resp = link_handler(
        state.clone(),
        PROVIDER.into(),
        headers,
        Some(axum::Json(LinkRequest {
            return_to: Some("/settings/accounts".into()),
        })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Pull the state out of the FakeProvider — `authorize_url`
    // recorded its inputs, the first of which is the `state` token.
    let seen = provider.authorize_seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 1, "authorize_url called once");
    let state_value = seen[0].0.clone();
    let flow = state
        .state_store
        .take(&state_value)
        .await
        .expect("state store take")
        .expect("flow present");
    assert_eq!(flow.link_mode_user_id.as_deref(), Some("u-link"));
    assert_eq!(flow.return_to.as_deref(), Some("/settings/accounts"));
    assert_eq!(flow.provider, PROVIDER);
}

#[tokio::test]
async fn link_without_session_is_unauthorized() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let resp = link_handler(state, PROVIDER.into(), HeaderMap::new(), None).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn link_with_absolute_return_to_is_rejected() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    let resp = link_handler(
        state,
        PROVIDER.into(),
        headers,
        Some(axum::Json(LinkRequest {
            return_to: Some("https://evil.example.com".into()),
        })),
    )
    .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn link_to_unknown_provider_is_404() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    let resp = link_handler(state, "nosuch".into(), headers, None).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn identities_lists_linked_providers_for_session_user() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    state
        .identity_store
        .insert(&OAuthIdentity {
            provider: PROVIDER.into(),
            provider_sub: "sub-1".into(),
            user_id: "u-1".into(),
            email: Some("ada@example.com".into()),
            display_name: Some("Ada".into()),
            linked_at: Utc::now(),
        })
        .await
        .unwrap();

    let resp = list_handler(state, headers).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::OK);
    // Shape sanity — the wire contract names these four keys; if
    // either disappears, every SPA built against the v0.1 endpoint
    // breaks on the next deploy.
    assert!(body.contains("\"provider\":\"github\""), "body={body}");
    assert!(body.contains("\"email\":\"ada@example.com\""));
    assert!(body.contains("\"display_name\":\"Ada\""));
    assert!(body.contains("\"last_login_at\""));
}

#[tokio::test]
async fn identities_without_session_is_unauthorized() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let resp = list_handler(state, HeaderMap::new()).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn identities_does_not_require_csrf_for_read_only_get() {
    // GET endpoints are not CSRF-protected (the double-submit cookie
    // pattern guards state-changing requests only). Strip the
    // `x-csrf-token` header and confirm the request still succeeds.
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let mut headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    headers.remove("x-csrf-token");
    let resp = list_handler(state, headers).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn unlink_unknown_provider_is_404_even_with_session() {
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", None).await;
    let resp = unlink_handler(state, "nosuch".into(), headers).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unlink_noop_when_no_identity_to_remove() {
    // The handler is idempotent: a user with a password and no
    // identities under `github` gets a 204, not a 404.
    let (_provider, me) = build().await;
    let state = Arc::new(me.state);
    let headers = seed_session(&state, "u-1", "ada@example.com", Some("$argon2id$dummy")).await;
    let resp = unlink_handler(state, PROVIDER.into(), headers).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}
