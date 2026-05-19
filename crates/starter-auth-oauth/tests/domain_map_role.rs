//! Phase 4 smoke test: Domain-map-assigns-Writer-to-acme-signups.
//!
//! `ProviderConfig.role_domain_map` is populated at startup from
//! `OAUTH_<PROVIDER>_ROLE_DOMAIN_MAP=acme.com=Writer,...`. On a
//! first-time OAuth signup the callback checks the verified email's
//! domain against the per-provider map and assigns the matched role,
//! falling back to `OAUTH_SIGNUP_DEFAULT_ROLE` on:
//!
//! - a domain not in the map,
//! - an empty / missing map for the provider,
//! - an unverified email (caught earlier by the verification guard).

#![cfg(feature = "sqlite")]

use std::collections::HashMap;
use std::sync::Arc;

use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::routes::{callback_handler, CallbackQuery, OAuthRoutesState};
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{OAuthFlowState, ProviderIdentity};
use starter_auth_users::Role;

const PROVIDER: &str = "github";

fn ident(sub: &str, email: &str) -> ProviderIdentity {
    ProviderIdentity {
        provider_sub: sub.into(),
        email: email.into(),
        email_verified: true,
        display_name: Some("Display".into()),
    }
}

async fn run_signup(state: OAuthRoutesState, state_value: &str) -> StatusCode {
    state
        .state_store
        .put(OAuthFlowState {
            provider: PROVIDER.into(),
            state: state_value.into(),
            pkce_verifier: "v".into(),
            return_to: None,
            link_mode_user_id: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();
    let resp = callback_handler(
        Arc::new(state),
        PROVIDER.into(),
        CallbackQuery {
            code: Some("c".into()),
            state: Some(state_value.into()),
            error: None,
            error_description: None,
        },
    )
    .await;
    resp.status()
}

#[tokio::test]
async fn acme_signup_gets_writer_role_from_domain_map() {
    let provider = FakeProvider::new(PROVIDER);
    provider.set_identity(ident("sub-acme", "alice@acme.com"));
    let mut me = MemoryEverything::new(vec![provider]).await;
    // `OAUTH_GITHUB_ROLE_DOMAIN_MAP=acme.com=Writer,evil.com=Reader`.
    let mut map = HashMap::new();
    map.insert("acme.com".to_string(), Role::Writer);
    map.insert("evil.com".to_string(), Role::Reader);
    me.state.role_domain_maps.insert(PROVIDER.to_string(), map);
    me.state.signup_default_role = Role::Reader;

    let user_store = me.state.user_store.clone();
    let status = run_signup(me.state, "s-acme").await;
    assert_eq!(status, StatusCode::FOUND);

    let created = user_store
        .find_by_email("alice@acme.com")
        .await
        .unwrap()
        .expect("user created");
    assert_eq!(
        created.role,
        Role::Writer,
        "acme.com domain must promote to Writer per role_domain_map",
    );
}

#[tokio::test]
async fn unmapped_domain_falls_back_to_default_role() {
    let provider = FakeProvider::new(PROVIDER);
    provider.set_identity(ident("sub-other", "carol@other.example"));
    let mut me = MemoryEverything::new(vec![provider]).await;
    let mut map = HashMap::new();
    map.insert("acme.com".to_string(), Role::Writer);
    me.state.role_domain_maps.insert(PROVIDER.to_string(), map);
    me.state.signup_default_role = Role::Reader;

    let user_store = me.state.user_store.clone();
    let status = run_signup(me.state, "s-other").await;
    assert_eq!(status, StatusCode::FOUND);
    let created = user_store
        .find_by_email("carol@other.example")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(created.role, Role::Reader);
}

#[tokio::test]
async fn empty_map_falls_back_to_default_role() {
    let provider = FakeProvider::new(PROVIDER);
    provider.set_identity(ident("sub-empty", "dave@acme.com"));
    let mut me = MemoryEverything::new(vec![provider]).await;
    // No entry inserted under PROVIDER — provider has no map.
    me.state.signup_default_role = Role::Writer; // verifies fallback uses *default*.

    let user_store = me.state.user_store.clone();
    let status = run_signup(me.state, "s-empty").await;
    assert_eq!(status, StatusCode::FOUND);
    let created = user_store
        .find_by_email("dave@acme.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(created.role, Role::Writer);
}

#[tokio::test]
async fn domain_match_is_case_insensitive() {
    // The provider returns an email with an upper-case domain; the
    // map was stored lowercased on parse. The lookup must
    // normalise the email side, not just the config side.
    let provider = FakeProvider::new(PROVIDER);
    provider.set_identity(ident("sub-case", "eve@ACME.com"));
    let mut me = MemoryEverything::new(vec![provider]).await;
    let mut map = HashMap::new();
    map.insert("acme.com".to_string(), Role::Writer);
    me.state.role_domain_maps.insert(PROVIDER.to_string(), map);
    me.state.signup_default_role = Role::Reader;

    let user_store = me.state.user_store.clone();
    let status = run_signup(me.state, "s-case").await;
    assert_eq!(status, StatusCode::FOUND);
    let created = user_store
        .find_by_email("eve@ACME.com")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(created.role, Role::Writer);
}
