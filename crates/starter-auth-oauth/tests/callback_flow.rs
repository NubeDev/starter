//! Seven-branch coverage of the OAuth callback handler.
//!
//! Each test wires `MemoryEverything` with a single `FakeProvider`,
//! seeds whatever DB state the branch needs (an identity row, a
//! local user, etc.), seeds a flow into the [`MemoryStateStore`],
//! and then calls `routes::callback_handler` directly. The handler
//! returns an `axum::response::Response`; we read status + body or
//! `Location` to assert the branch landed where we expected.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{
    routes::{callback_handler, CallbackQuery, OAuthRoutesState},
    OAuthFlowState, OAuthIdentity, ProviderError, ProviderIdentity,
};
use starter_auth_users::Role;

const PROVIDER: &str = "github";

async fn put_flow(state: &OAuthRoutesState, state_value: &str, link_user: Option<&str>) {
    state
        .state_store
        .put(OAuthFlowState {
            provider: PROVIDER.into(),
            state: state_value.into(),
            pkce_verifier: "v-deadbeef".into(),
            return_to: Some("/after".into()),
            link_mode_user_id: link_user.map(|s| s.to_string()),
            created_at: Utc::now(),
        })
        .await
        .expect("put flow");
}

fn callback(state_value: &str) -> CallbackQuery {
    CallbackQuery {
        code: Some("auth-code-1".into()),
        state: Some(state_value.into()),
        error: None,
        error_description: None,
    }
}

fn ident(sub: &str, email: &str, verified: bool) -> ProviderIdentity {
    ProviderIdentity {
        provider_sub: sub.into(),
        email: email.into(),
        email_verified: verified,
        display_name: Some("Display".into()),
    }
}

async fn body_text(resp: axum::response::Response) -> (StatusCode, String) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn build() -> (Arc<FakeProvider>, MemoryEverything) {
    let provider = FakeProvider::new(PROVIDER);
    let me = MemoryEverything::new(vec![provider.clone()]).await;
    (provider, me)
}

#[tokio::test]
async fn branch_1_signin_hit_existing_identity() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-1", "ada@example.com", None, Role::Reader)
        .await
        .unwrap();
    me.state
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
    provider.set_identity(ident("sub-1", "ada@example.com", true));

    put_flow(&me.state, "s1", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s1")).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
    assert_eq!(
        resp.headers().get(axum::http::header::LOCATION).unwrap(),
        "/after"
    );
    // Session cookie minted (Hard rule R1).
    let cookies = resp
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .map(|h| h.to_str().unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(cookies
        .iter()
        .any(|c| c.starts_with("starter_session=sas_")));
    assert!(cookies.iter().any(|c| c.starts_with("starter_csrf=")));
}

#[tokio::test]
async fn branch_2_link_hit_same_user_is_idempotent_signin() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-1", "ada@example.com", None, Role::Reader)
        .await
        .unwrap();
    me.state
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
    provider.set_identity(ident("sub-1", "ada@example.com", true));

    put_flow(&me.state, "s2", Some("u-1")).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s2")).await;
    assert_eq!(resp.status(), StatusCode::FOUND);
}

#[tokio::test]
async fn branch_2_link_hit_other_user_refuses() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-owner", "owner@example.com", None, Role::Reader)
        .await
        .unwrap();
    me.state
        .user_store
        .create("u-other", "other@example.com", None, Role::Reader)
        .await
        .unwrap();
    me.state
        .identity_store
        .insert(&OAuthIdentity {
            provider: PROVIDER.into(),
            provider_sub: "sub-1".into(),
            user_id: "u-owner".into(),
            email: Some("owner@example.com".into()),
            display_name: None,
            linked_at: Utc::now(),
        })
        .await
        .unwrap();
    provider.set_identity(ident("sub-1", "owner@example.com", true));

    put_flow(&me.state, "s2b", Some("u-other")).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s2b")).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.contains("already_linked_to_other_user"));
}

#[tokio::test]
async fn branch_3_link_miss_inserts_for_logged_in_user() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-1", "ada@example.com", None, Role::Reader)
        .await
        .unwrap();
    provider.set_identity(ident("sub-new", "secondary@example.com", true));

    let id_store = me.state.identity_store.clone();
    put_flow(&me.state, "s3", Some("u-1")).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s3")).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let row = id_store.find(PROVIDER, "sub-new").await.unwrap().unwrap();
    assert_eq!(row.user_id, "u-1");
}

#[tokio::test]
async fn branch_4_signin_miss_verified_email_match_links() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-1", "ada@example.com", None, Role::Reader)
        .await
        .unwrap();
    provider.set_identity(ident("sub-new", "ada@example.com", true));

    let id_store = me.state.identity_store.clone();
    put_flow(&me.state, "s4", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s4")).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let row = id_store.find(PROVIDER, "sub-new").await.unwrap().unwrap();
    assert_eq!(row.user_id, "u-1");
}

#[tokio::test]
async fn branch_5_signin_miss_unverified_collision_refuses() {
    let (provider, me) = build().await;
    me.state
        .user_store
        .create("u-1", "ada@example.com", None, Role::Reader)
        .await
        .unwrap();
    // Unverified email matching an existing user is the canonical
    // account-takeover vector (Hard rule R3).
    provider.set_identity(ident("sub-attacker", "ada@example.com", false));

    let id_store = me.state.identity_store.clone();
    put_flow(&me.state, "s5", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s5")).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.contains("email_already_registered"));

    // Critical assertion: no identity row was created.
    assert!(id_store
        .find(PROVIDER, "sub-attacker")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn branch_6_signup_enabled_creates_user_and_identity() {
    let (provider, me) = build().await;
    provider.set_identity(ident("sub-new", "fresh@example.com", true));

    let user_store = me.state.user_store.clone();
    let id_store = me.state.identity_store.clone();
    put_flow(&me.state, "s6", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s6")).await;
    assert_eq!(resp.status(), StatusCode::FOUND);

    let created = user_store
        .find_by_email("fresh@example.com")
        .await
        .unwrap()
        .expect("user created");
    // OAuth-only user — no local password (Hard rule R8).
    assert!(created.password_hash.is_none());
    assert_eq!(created.role, Role::Reader);
    let row = id_store.find(PROVIDER, "sub-new").await.unwrap().unwrap();
    assert_eq!(row.user_id, created.id);
}

#[tokio::test]
async fn branch_7_signup_disabled_refuses_first_time_callback() {
    let provider = FakeProvider::new(PROVIDER);
    let mut me = MemoryEverything::new(vec![provider.clone()]).await;
    me.state.signup_enabled = false;
    provider.set_identity(ident("sub-new", "fresh@example.com", true));

    put_flow(&me.state, "s7", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("s7")).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert!(body.contains("signup_disabled"));
}

#[tokio::test]
async fn callback_with_missing_state_fails_without_provider_io() {
    let (provider, me) = build().await;
    let resp = callback_handler(
        Arc::new(me.state),
        PROVIDER.into(),
        CallbackQuery {
            code: Some("auth-code".into()),
            state: Some("nonexistent".into()),
            error: None,
            error_description: None,
        },
    )
    .await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("sign_in_failed"));
    // Provider was never called (Hard rule R5 — no DB write before
    // identity is known, and no provider IO either).
    assert!(provider.seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn callback_propagates_provider_unverified_email_as_failure() {
    let (provider, me) = build().await;
    provider.set_error(ProviderError::UnverifiedEmail);
    put_flow(&me.state, "snv", None).await;
    let resp = callback_handler(Arc::new(me.state), PROVIDER.into(), callback("snv")).await;
    let (status, body) = body_text(resp).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("sign_in_failed"));
    assert!(body.contains("correlation_id"));
}
