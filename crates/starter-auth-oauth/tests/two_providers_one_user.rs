//! Phase 2 smoke test: **same-human-two-providers-one-user**.
//!
//! Alice signs up via Google with a verified `alice@example.com`,
//! then later returns and signs in via GitHub which independently
//! verifies the same address. The GitHub callback finds no
//! `(github, gh_sub)` identity row, falls into Branch 4 of the
//! callback decision tree (`find_by_email` hit + `email_verified =
//! true`), and links GitHub to Alice's existing user.
//!
//! If a *second* `users` row is created instead, the verified-email
//! linking rule has slipped — the test asserts on the user count and
//! on the user id shared by both identity rows so that regression
//! shows up as a hard failure rather than a subtle data drift.

#![cfg(feature = "sqlite")]

use std::sync::Arc;

use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{
    routes::{callback_handler, CallbackQuery, OAuthRoutesState},
    OAuthFlowState, ProviderIdentity,
};

const EMAIL: &str = "alice@example.com";

fn ident(sub: &str) -> ProviderIdentity {
    // Verified because both providers, after their respective
    // verification paths (GitHub `/user/emails`, Google
    // `email_verified` claim), report ownership of EMAIL.
    ProviderIdentity {
        provider_sub: sub.into(),
        email: EMAIL.into(),
        email_verified: true,
        display_name: Some("Alice".into()),
    }
}

async fn put_flow(state: &OAuthRoutesState, provider: &str, value: &str) {
    state
        .state_store
        .put(OAuthFlowState {
            provider: provider.into(),
            state: value.into(),
            pkce_verifier: "v-deadbeef".into(),
            return_to: Some("/after".into()),
            link_mode_user_id: None,
            created_at: Utc::now(),
        })
        .await
        .expect("put flow");
}

fn callback(state_value: &str) -> CallbackQuery {
    CallbackQuery {
        code: Some("auth-code".into()),
        state: Some(state_value.into()),
        error: None,
        error_description: None,
    }
}

#[tokio::test]
async fn google_signup_then_github_signin_lands_on_same_user() {
    // Two FakeProviders wearing the production-shaped ids; the
    // callback handler keys identity rows on those strings, so the
    // string values matter even though the impls are fakes.
    let google = FakeProvider::new("google");
    let github = FakeProvider::new("github");

    let me = MemoryEverything::new(vec![google.clone(), github.clone()]).await;
    let state = Arc::new(me.state);

    // --- Round 1: Google signup. No user exists yet; Branch 6
    // (`find_by_email` miss + signup_enabled) creates Alice.
    google.set_identity(ident("google-sub-alice"));
    put_flow(&state, "google", "g-state-1").await;
    let resp = callback_handler(state.clone(), "google".into(), callback("g-state-1")).await;
    assert_eq!(
        resp.status(),
        StatusCode::FOUND,
        "google signup should redirect on success"
    );

    let alice = state
        .user_store
        .find_by_email(EMAIL)
        .await
        .unwrap()
        .expect("alice exists after google signup");
    let google_row = state
        .identity_store
        .find("google", "google-sub-alice")
        .await
        .unwrap()
        .expect("google identity row written");
    assert_eq!(google_row.user_id, alice.id);

    // --- Round 2: GitHub sign-in. No `(github, gh_sub)` row, but
    // GitHub returns the same verified email. Branch 4 of the
    // callback tree should link this identity to Alice's existing
    // user rather than spawning a second `users` row.
    github.set_identity(ident("github-sub-alice"));
    put_flow(&state, "github", "gh-state-1").await;
    let resp = callback_handler(state.clone(), "github".into(), callback("gh-state-1")).await;
    assert_eq!(
        resp.status(),
        StatusCode::FOUND,
        "github sign-in should redirect after auto-link"
    );

    let github_row = state
        .identity_store
        .find("github", "github-sub-alice")
        .await
        .unwrap()
        .expect("github identity row written");

    // The whole point of the test: both identities resolve to the
    // **same** local user.
    assert_eq!(
        github_row.user_id, alice.id,
        "verified-email auto-link must reuse the existing user; \
         a different id here means R3's linking rule has slipped"
    );

    // Belt-and-suspenders: only one user exists. The callback
    // handler currently has no public "count users" hook, so we
    // probe `find_by_email` for the only address in play and
    // confirm the id is unchanged.
    let still_alice = state
        .user_store
        .find_by_email(EMAIL)
        .await
        .unwrap()
        .expect("alice still resolves");
    assert_eq!(still_alice.id, alice.id);
}
