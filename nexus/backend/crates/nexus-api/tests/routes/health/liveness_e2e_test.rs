//! `GET /livez` and `GET /readyz` (WS-16) end-to-end through the router.
//!
//! `/livez` reads the runtime canary; these tests spawn the real canary tick
//! task (via `state_with_canary`) so the atomic stays fresh and a healthy
//! process reports 200 `live`. `/readyz` additionally runs a `SELECT 1` against
//! the metadata pool, so it needs a real database. Both routes are
//! unauthenticated — no principal layer, no token — which these tests prove by
//! hitting them with no auth.

#![cfg(feature = "testing")]

use std::sync::Arc;
use std::time::Duration;

use nexus_api::middleware::StreamTokenSigner;
use nexus_api::serve;
use nexus_api::state::AppState;
use nexus_engine::{FlowManager, LiveRunner};
use nexus_store::datasource::Envelope;
use nexus_store::testing::runtime_pool;
use nexus_store::QueryGuards;
use starter_authz::testing::AllowAll;
use starter_server::testing::TestApp;
use starter_store_postgres::testing::with_database;

/// Build state with a *running* canary so `/livez` reflects a live runtime for
/// the whole test (a bare `Canary::new()` would go stale after the 5s budget if
/// the test ran long). Returns the tick `JoinHandle`; the caller keeps it alive.
fn state_with_canary(pool: &sqlx::PgPool) -> (AppState, tokio::task::JoinHandle<()>) {
    let (canary, tick) = nexus_api::boot::runtime_canary::spawn();
    let mut st = state(pool);
    st.canary = canary;
    (st, tick)
}

fn state(pool: &sqlx::PgPool) -> AppState {
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
        sessions: nexus_api::agents::SessionRunner::new(
            std::env::temp_dir().join("nexus-knowledge-test"),
            nexus_skills::BrevityMode::Off,
        ),
        stream_signer: StreamTokenSigner::new(*b"test-stream-key-0123456789abcdef"),
        stream_token_ttl: Duration::from_secs(60),
        engine: Arc::new(AllowAll),
        kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        extension_kinds: Arc::new(nexus_api::kinds::Registry::empty()),
        datasource_kinds: Arc::new(nexus_api::datasource_kinds::Registry::empty()),
        prefs: nexus_api::prefs::prefs_store(pool.clone()),
        changelog: nexus_api::changelog::ChangelogHandles::new(
            pool.clone(),
            Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap(),
        ),
        query_cache: nexus_api::cache::CacheConfig::default().build(),
        quotas: nexus_api::quota::TenantQuotas::new(nexus_api::quota::QuotaConfig::default()),
        rate_limiter: nexus_api::ratelimit::TenantRateLimiter::new(
            nexus_api::ratelimit::RateLimitConfig::default(),
        ),
        canary: Default::default(),
    }
}

async fn app_for(pool: &sqlx::PgPool) -> (TestApp, tokio::task::JoinHandle<()>) {
    let (st, tick) = state_with_canary(pool);
    (TestApp::spawn(serve::router(st)).await, tick)
}

#[tokio::test]
async fn livez_reports_live_for_a_fresh_runtime() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let (app, _tick) = app_for(&pg).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/livez", app.base_url))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 200, "a fresh canary must report live");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "live");
}

#[tokio::test]
async fn readyz_reports_ready_when_db_answers() {
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let (app, _tick) = app_for(&pg).await;

    let resp = reqwest::Client::new()
        .get(format!("{}/readyz", app.base_url))
        .send()
        .await
        .expect("request");

    assert_eq!(resp.status(), 200, "live runtime + reachable DB == ready");
    let body: serde_json::Value = resp.json().await.expect("json");
    assert_eq!(body["status"], "ready");
}

#[tokio::test]
async fn liveness_probes_need_no_auth() {
    // No principal layer, no bearer token: the probes must still answer. (The
    // `serve::router` helper deliberately mounts the product router without the
    // identity layer, so this is the unauthenticated path.)
    let (admin, _guard) = with_database().await;
    let pg = runtime_pool(admin.sqlx()).await;
    let (app, _tick) = app_for(&pg).await;

    for path in ["/livez", "/readyz"] {
        let resp = reqwest::Client::new()
            .get(format!("{}{}", app.base_url, path))
            .send()
            .await
            .expect("request");
        assert_eq!(resp.status(), 200, "{path} must be reachable without auth");
    }
}
