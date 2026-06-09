//! Agent acceptance: an agent is created, edited, listed, and run over the API,
//! and a session's durable lifecycle is observable. Mirrors `flow_e2e_test.rs`.
//!
//! The model call itself needs a provider key and network, which CI lacks, so
//! this test asserts the *control-plane* behaviour that is deterministic without
//! one: CRUD, the session row transitioning `running` → terminal, the SSE token
//! being minted, and grant gating. The inference output is not asserted.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use axum::Extension;
use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use serde_json::{json, Value};
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_spi::auth::{Principal, Role};
use starter_store_postgres::testing::with_database;

fn test_state(pool: &sqlx::PgPool) -> AppState {
    AppState {
        metadata: pool.clone(),
        datasource: pool.clone(),
        datasource_pools: Default::default(),
        envelope: Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        guards: QueryGuards {
            statement_timeout: Duration::from_secs(5),
            max_rows: 1000,
            max_bytes: 8 * 1024 * 1024,
        },
        live: LiveRunner::new().expect("engine init"),
        flows: FlowManager::new().expect("flow manager init"),
        sessions: nexus_api::agents::SessionRunner::new(std::env::temp_dir().join("nexus-knowledge-test"), nexus_skills::BrevityMode::Off),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        ),
    }
}

fn acme_admin() -> Principal {
    Principal {
        subject: "alice".into(),
        role: Role::Admin,
        scopes: vec![],
        tenant_id: Some("acme".into()),
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: Value::Null,
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn agent_crud_and_session_lifecycle() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // --- create -----------------------------------------------------------
    let created: Value = client
        .post(format!("{}/api/v1/agents", app.base_url))
        .json(&json!({
            "name": "summariser",
            "backend": "anthropic",
            "model": "small",
            "system_prompt": "You summarise tersely.",
            "config": { "temperature": 0.2 }
        }))
        .send()
        .await
        .expect("create")
        .json()
        .await
        .expect("body");
    let agent_id = created["id"].as_str().expect("id").to_string();
    assert_eq!(created["backend"], "anthropic");
    assert_eq!(created["model"], "small");

    // A duplicate name in the tenant is a conflict.
    let dup = client
        .post(format!("{}/api/v1/agents", app.base_url))
        .json(&json!({ "name": "summariser", "backend": "anthropic" }))
        .send()
        .await
        .expect("dup");
    assert_eq!(dup.status(), 409);

    // --- list / get -------------------------------------------------------
    let list: Value = client
        .get(format!("{}/api/v1/agents", app.base_url))
        .send()
        .await
        .expect("list")
        .json()
        .await
        .expect("body");
    assert_eq!(list.as_array().expect("array").len(), 1);

    // --- update -----------------------------------------------------------
    let updated: Value = client
        .put(format!("{}/api/v1/agents/{agent_id}", app.base_url))
        .json(&json!({ "model": "large" }))
        .send()
        .await
        .expect("update")
        .json()
        .await
        .expect("body");
    assert_eq!(updated["model"], "large");
    // Unchanged fields are preserved by the partial update.
    assert_eq!(updated["backend"], "anthropic");

    // --- start a session --------------------------------------------------
    let session: Value = client
        .post(format!("{}/api/v1/agents/{agent_id}/sessions", app.base_url))
        .json(&json!({ "prompt": "Say hi." }))
        .send()
        .await
        .expect("session")
        .json()
        .await
        .expect("body");
    let session_id = session["id"].as_str().expect("session id").to_string();
    assert_eq!(session["status"], "running");
    // A signed SSE token is minted for the feed.
    assert!(
        !session["token"].as_str().unwrap_or_default().is_empty(),
        "session response carries an SSE token"
    );

    // The session is listed under its agent.
    let sessions: Value = client
        .get(format!("{}/api/v1/agents/{agent_id}/sessions", app.base_url))
        .send()
        .await
        .expect("list sessions")
        .json()
        .await
        .expect("body");
    assert_eq!(sessions.as_array().expect("array").len(), 1);

    // The run resolves to a terminal state (no provider key in CI, so it fails
    // fast rather than completing — either terminal state is acceptable, what
    // matters is that the durable row leaves `running`).
    let mut status = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while tokio::time::Instant::now() < deadline {
        let s: Value = client
            .get(format!("{}/api/v1/agents/sessions/{session_id}", app.base_url))
            .send()
            .await
            .expect("get session")
            .json()
            .await
            .expect("body");
        status = s["status"].as_str().unwrap_or_default().to_string();
        if status != "running" {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(
        matches!(status.as_str(), "completed" | "failed"),
        "session reached a terminal state, got {status:?}"
    );

    // --- delete -----------------------------------------------------------
    let del = client
        .delete(format!("{}/api/v1/agents/{agent_id}", app.base_url))
        .send()
        .await
        .expect("delete");
    assert_eq!(del.status(), 204);

    // Its sessions cascaded with it.
    let gone = client
        .get(format!("{}/api/v1/agents/sessions/{session_id}", app.base_url))
        .send()
        .await
        .expect("get gone");
    assert_eq!(gone.status(), 404);

    drop(app);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn agent_session_sse_token_is_required() {
    let (admin, _guard) = with_database().await;
    let pool = runtime_pool(admin.sqlx()).await;

    let router = serve::router(test_state(&pool)).layer(Extension(acme_admin()));
    let app = TestApp::spawn(router).await;
    let client = reqwest::Client::new();

    // A forged/absent token on the SSE feed is rejected, never serving events.
    let no_token = client
        .get(format!(
            "{}/api/v1/agents/sessions/{}/events",
            app.base_url,
            uuid::Uuid::nil()
        ))
        .send()
        .await
        .expect("no token");
    // Missing required query param → 400 from extraction; a present-but-bad token
    // → 401. Either way the feed is not served.
    assert!(
        no_token.status() == 400 || no_token.status() == 401,
        "SSE feed refuses an unauthenticated request, got {}",
        no_token.status()
    );

    let bad_token = client
        .get(format!(
            "{}/api/v1/agents/sessions/{}/events?token=not-a-real-token",
            app.base_url,
            uuid::Uuid::nil()
        ))
        .send()
        .await
        .expect("bad token");
    assert_eq!(bad_token.status(), 401);

    drop(app);
}
