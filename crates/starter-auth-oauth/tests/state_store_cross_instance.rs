//! Phase 4 smoke test: Callback-survives-wrong-instance-routing.
//!
//! Two `OAuthRoutesState`s share the same sqlite pool but each has
//! its own `FakeProvider` instance + own `IdentityStore`/`UserStore`
//! handle (still pointing at the shared pool). With
//! [`SqliteStateStore`] wired into both, the flow `put` on instance
//! A is visible to instance B's `take` on callback, so the user
//! lands signed-in even though the load balancer routed the two
//! halves of the redirect dance to different processes.
//!
//! The same scenario with [`MemoryStateStore`] (per-process) must
//! fail fast with the "state token not found" path — `BAD_REQUEST`
//! and a structured `sign_in_failed` body — *not* a 500.

#![cfg(feature = "sqlite")]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use axum::body::to_bytes;
use axum::http::StatusCode;
use chrono::Utc;
use starter_auth_oauth::routes::{callback_handler, CallbackQuery, OAuthRoutesState};
use starter_auth_oauth::testing::{FakeProvider, MemoryEverything};
use starter_auth_oauth::{
    MemoryStateStore, OAuthFlowState, OAuthStateStore, ProviderIdentity, SqliteIdentityStore,
    SqliteStateStore,
};
use starter_auth_users::store::{SqliteSessionStore, SqliteUserStore};
use starter_auth_users::Role;
use starter_store_sqlite::Pool;

const PROVIDER: &str = "github";

/// Build a second `OAuthRoutesState` that shares the supplied
/// pool but otherwise looks like a fresh process — its own provider
/// arc, its own store handles. The two states only have the
/// underlying tables in common.
fn instance_b(
    pool: Pool,
    state_store: Arc<dyn OAuthStateStore>,
    provider: Arc<FakeProvider>,
) -> OAuthRoutesState {
    let user_store = Arc::new(SqliteUserStore::new(pool.clone()));
    let session_store = Arc::new(SqliteSessionStore::new(pool.clone()));
    let identity_store = Arc::new(SqliteIdentityStore::new(pool));
    let mut providers: BTreeMap<String, Arc<dyn starter_auth_oauth::OAuthProvider>> =
        BTreeMap::new();
    providers.insert(PROVIDER.into(), provider);
    OAuthRoutesState {
        providers,
        state_store,
        identity_store,
        user_store,
        session_store,
        base_url: "https://app.example.com".to_string(),
        signup_enabled: true,
        signup_default_role: Role::Reader,
        role_domain_maps: HashMap::new(),
        default_return_to: "/".to_string(),
    }
}

fn ident_for(sub: &str, email: &str) -> ProviderIdentity {
    ProviderIdentity {
        provider_sub: sub.into(),
        email: email.into(),
        email_verified: true,
        display_name: Some("Display".into()),
    }
}

async fn body_text(resp: axum::response::Response) -> (StatusCode, String) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 64 * 1024).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn sqlite_state_store_handoff_between_instances() {
    // Instance A wires the routes + identity stores + a memory state
    // store that the wiring helper builds by default — we swap it
    // out below so both instances share one `SqliteStateStore`.
    let provider_a = FakeProvider::new(PROVIDER);
    let me = MemoryEverything::new(vec![provider_a.clone()]).await;
    let pool = me.pool.clone();
    let shared_state_store: Arc<dyn OAuthStateStore> =
        Arc::new(SqliteStateStore::new(pool.clone()));

    // Replace instance A's state store with the shared sqlite one
    // and start the flow there.
    let mut state_a = me.state;
    state_a.state_store = shared_state_store.clone();
    state_a
        .state_store
        .put(OAuthFlowState {
            provider: PROVIDER.into(),
            state: "shared-state-1".into(),
            pkce_verifier: "v-shared".into(),
            return_to: Some("/after-handoff".into()),
            link_mode_user_id: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    // Instance B is a separate `OAuthRoutesState` — different
    // provider arc, different store handles — that only knows about
    // the shared pool and the shared state store. This is what
    // happens after a load-balancer reroutes the callback.
    let provider_b = FakeProvider::new(PROVIDER);
    provider_b.set_identity(ident_for("sub-shared", "alice@example.com"));
    let state_b = instance_b(pool.clone(), shared_state_store.clone(), provider_b.clone());

    let resp = callback_handler(
        Arc::new(state_b),
        PROVIDER.into(),
        CallbackQuery {
            code: Some("code-shared".into()),
            state: Some("shared-state-1".into()),
            error: None,
            error_description: None,
        },
    )
    .await;

    assert_eq!(
        resp.status(),
        StatusCode::FOUND,
        "instance B should resolve the flow A started"
    );
    assert_eq!(
        resp.headers().get(axum::http::header::LOCATION).unwrap(),
        "/after-handoff"
    );

    // Sanity: instance B's provider was the one that did the
    // network round-trip — confirms we exercised B end-to-end.
    assert_eq!(provider_b.seen.lock().unwrap().len(), 1);
    assert!(
        provider_a.seen.lock().unwrap().is_empty(),
        "instance A must not have been touched after its `put`",
    );

    // Single-use: the same state value cannot be replayed.
    assert!(shared_state_store
        .take("shared-state-1")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn memory_state_store_fails_fast_when_callback_lands_on_wrong_instance() {
    // Same scenario as above with one swap: each instance carries
    // its own `MemoryStateStore`. The state-token stored on A is
    // invisible to B, so B's callback must surface the "state token
    // not found" path — sign_in_failed at BAD_REQUEST, not a 500.
    let provider_a = FakeProvider::new(PROVIDER);
    let me = MemoryEverything::new(vec![provider_a.clone()]).await;
    let pool = me.pool.clone();

    let state_a = me.state;
    state_a
        .state_store
        .put(OAuthFlowState {
            provider: PROVIDER.into(),
            state: "iso-state".into(),
            pkce_verifier: "v-iso".into(),
            return_to: None,
            link_mode_user_id: None,
            created_at: Utc::now(),
        })
        .await
        .unwrap();

    let provider_b = FakeProvider::new(PROVIDER);
    provider_b.set_identity(ident_for("sub-iso", "bob@example.com"));
    // A fresh MemoryStateStore — the per-process default — that
    // never saw A's `put`.
    let state_b = instance_b(pool, Arc::new(MemoryStateStore::new()), provider_b.clone());

    let resp = callback_handler(
        Arc::new(state_b),
        PROVIDER.into(),
        CallbackQuery {
            code: Some("code-iso".into()),
            state: Some("iso-state".into()),
            error: None,
            error_description: None,
        },
    )
    .await;

    let (status, body) = body_text(resp).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "wrong-instance must not 500"
    );
    assert!(
        body.contains("sign_in_failed"),
        "user-facing body is sign_in_failed (not the internal reason): {body}",
    );

    // Belt-and-suspenders: B never reached the provider IO step.
    assert!(provider_b.seen.lock().unwrap().is_empty());
}
