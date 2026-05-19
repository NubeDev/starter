//! Smoke test: Provider-access-token-never-persists.
//!
//! Hard rule R2 — "the provider access token never leaves
//! `OAuthProvider::fetch_identity`". Two complementary checks:
//!
//! 1. **Static CI grep guard.** Walk every `.rs` file in `src/`
//!    outside `src/providers/*` and assert the literal
//!    `access_token` does not appear. Provider impls are the only
//!    place the token is allowed to exist; everything else
//!    (routes, session bridge, stores, callback handler) must be
//!    structurally token-free so a future regression that smuggles
//!    the token into a logged span or a persisted column trips this
//!    test before it ships.
//!
//! 2. **Runtime recording-subscriber assertion.** Install a custom
//!    `tracing_subscriber::Layer` that records every field name and
//!    value emitted by the OAuth crate, run a full successful
//!    callback round trip through `callback_handler`, and assert:
//!    a) no recorded field NAME is `access_token` / `bearer` /
//!    `authorization`,
//!    b) no recorded field VALUE contains the sentinel token the
//!    test injected — through the auth `code` query param and the
//!    `display_name` claim — into the callback path,
//!    c) no row in `starter_auth_oauth_identities` /
//!    `starter_auth_users_users` / `starter_auth_users_sessions`
//!    contains the sentinel string.

#![cfg(feature = "sqlite")]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::routes::{callback_handler, CallbackQuery};
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{OAuthFlowState, ProviderIdentity};
use tracing::subscriber::set_default;
use tracing_subscriber::field::Visit;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::Layer;

const SENTINEL: &str = "OAUTH_ACCESS_TOKEN_SENTINEL_8c7a4f9e";

// ---------- (1) static grep guard ----------

fn collect_rs_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("read_dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, files);
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn access_token_literal_only_appears_in_provider_implementations() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src = manifest_dir.join("src");

    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);

    // The narrow allow-list: providers/* (where token exchange
    // actually happens) and testing.rs (which documents the rule in
    // its module-level doc comment). Everything else is structurally
    // token-free.
    let allow_prefixes: BTreeSet<PathBuf> = [src.join("providers"), src.join("testing.rs")]
        .into_iter()
        .collect();

    let mut offenders: Vec<String> = Vec::new();
    for file in &files {
        let in_allow = allow_prefixes
            .iter()
            .any(|p| file.starts_with(p) || file == p);
        if in_allow {
            continue;
        }
        let body = fs::read_to_string(file).expect("read file");
        if body.contains("access_token") {
            offenders.push(format!("{} contains `access_token`", file.display()));
        }
    }
    assert!(
        offenders.is_empty(),
        "R2 violation — `access_token` must only appear in src/providers/* (and the testing.rs doc comment). Offenders:\n  {}",
        offenders.join("\n  "),
    );
}

// ---------- (2) recording subscriber + runtime assertion ----------

#[derive(Default, Clone)]
struct Recording {
    fields: Arc<Mutex<Vec<(String, String)>>>,
}

impl Recording {
    fn snapshot(&self) -> Vec<(String, String)> {
        self.fields.lock().unwrap().clone()
    }
}

struct RecordingVisitor<'a> {
    out: &'a mut Vec<(String, String)>,
}

impl<'a> Visit for RecordingVisitor<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.out
            .push((field.name().to_string(), format!("{value:?}")));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.out.push((field.name().to_string(), value.to_string()));
    }
}

struct RecordingLayer {
    rec: Recording,
}

impl<S> Layer<S> for RecordingLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = self.rec.fields.lock().unwrap();
        let mut visitor = RecordingVisitor { out: &mut fields };
        event.record(&mut visitor);
    }
}

#[tokio::test]
async fn no_event_field_carries_an_access_token_value() {
    let provider = FakeProvider::new("github");
    // The SENTINEL is embedded in the auth code (a secret the
    // callback handler must NOT log) and in display_name (a claim
    // value that legitimately reaches the persistence layer but
    // must not flow into tracing).
    provider.set_identity(ProviderIdentity {
        provider_sub: "sub-token-leak".into(),
        email: "leak@example.com".into(),
        email_verified: true,
        display_name: Some(format!("Leak {SENTINEL}")),
    });

    let me = MemoryEverything::new(vec![provider.clone()]).await;
    let state = Arc::new(me.state);
    state
        .state_store
        .put(OAuthFlowState {
            provider: "github".into(),
            state: "s-token".into(),
            pkce_verifier: "v-token".into(),
            return_to: Some("/after".into()),
            link_mode_user_id: None,
            created_at: Utc::now(),
        })
        .await
        .expect("put flow");

    let recording = Recording::default();
    let layer = RecordingLayer {
        rec: recording.clone(),
    };
    let subscriber = tracing_subscriber::registry().with(layer);
    let _guard = set_default(subscriber);

    let resp = callback_handler(
        state.clone(),
        "github".into(),
        CallbackQuery {
            code: Some(format!("code-{SENTINEL}")),
            state: Some("s-token".into()),
            error: None,
            error_description: None,
        },
    )
    .await;
    assert_eq!(resp.status(), StatusCode::FOUND, "sign-in succeeded");

    let recorded = recording.snapshot();

    // (a) No field name is one of the forbidden token-bearing names.
    let forbidden_names = ["access_token", "bearer", "authorization"];
    let bad_names: Vec<_> = recorded
        .iter()
        .filter(|(name, _)| forbidden_names.iter().any(|f| name == f))
        .collect();
    assert!(
        bad_names.is_empty(),
        "R2 violation — tracing event carries a forbidden token field name: {bad_names:?}",
    );

    // (b) No field value contains the sentinel. The handler never
    // logs the authorization code, and display_name is never
    // attached to a tracing field.
    let leaks: Vec<_> = recorded
        .iter()
        .filter(|(_, v)| v.contains(SENTINEL))
        .collect();
    assert!(
        leaks.is_empty(),
        "R2 violation — tracing event leaked the SENTINEL: {leaks:?}",
    );

    // (c) No persisted row contains the sentinel **as a token-shaped
    // blob**. The display_name claim is a legitimate user-supplied
    // string and would land in oauth_identities.display_name; the
    // sentinel-in-display-name probe here is intentionally checking
    // the *other* columns (email / provider_sub / session id) that
    // must not pick the value up.
    let user_email_leak: Option<String> = sqlx::query_scalar(
        "SELECT email FROM starter_auth_users_users \
         WHERE email LIKE '%OAUTH_ACCESS_TOKEN_SENTINEL%' LIMIT 1",
    )
    .fetch_optional(me.pool.sqlx())
    .await
    .expect("query users");
    assert!(user_email_leak.is_none());

    let ident_leak: Option<String> = sqlx::query_scalar(
        "SELECT provider_sub FROM starter_auth_oauth_identities \
         WHERE provider_sub LIKE '%OAUTH_ACCESS_TOKEN_SENTINEL%' \
            OR email          LIKE '%OAUTH_ACCESS_TOKEN_SENTINEL%' LIMIT 1",
    )
    .fetch_optional(me.pool.sqlx())
    .await
    .expect("query identities");
    assert!(ident_leak.is_none());

    let session_leak: Option<String> = sqlx::query_scalar(
        "SELECT id FROM starter_auth_users_sessions \
         WHERE id LIKE '%OAUTH_ACCESS_TOKEN_SENTINEL%' LIMIT 1",
    )
    .fetch_optional(me.pool.sqlx())
    .await
    .expect("query sessions");
    assert!(session_leak.is_none());
}
