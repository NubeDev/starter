//! Phase 7d.2 smoke tests — adapter-surface labelling on the
//! decision audit log. SCOPE-EXT.md §5.
//!
//! The engine reads its [`DecisionEntry::surface`] from a tokio
//! task-local that each adapter binds around its `engine.check()`
//! call. These tests exercise the task-local directly (no axum /
//! tonic / MCP transport in the loop) because the assertion is on
//! the **engine + sink** contract: a deny issued with a surface
//! bound lands in `starter_authz_decisions.surface` distinguishable
//! from a deny issued with a different surface bound.
//!
//! The wire-side tests (REST middleware sets `"rest"`,
//! `AuthzedToolBinding` sets `"mcp"`, `ExtensionGrpcService` sets
//! `"grpc"`) live next to each adapter; here we prove the engine
//! end of the contract.

#![cfg(feature = "sqlite")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use starter_authz::audit::DbDecisionSink;
use starter_authz::audit::DecisionSinkConfig;
use starter_authz::store::AUTHZ_SQLITE_MIGRATOR;
use starter_authz::{with_surface, AuthzConfig, DecisionSink, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec};
use starter_store_sqlite::{migrate, migrate::MigrationSource, testing::ephemeral, Pool};

async fn fresh_pool() -> Pool {
    let pool = ephemeral().await;
    migrate(&pool)
        .with_source(MigrationSource {
            name: "starter_authz",
            migrator: &AUTHZ_SQLITE_MIGRATOR,
        })
        .run()
        .await
        .expect("migrations apply");
    pool
}

fn reader(subject: &str) -> Principal {
    Principal {
        subject: subject.into(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: None,
        teams: vec![],
        extra: serde_json::Value::Null,
    }
}

async fn count_surface(pool: &Pool, surface: &str) -> i64 {
    use sqlx::Row;
    let r = sqlx::query(
        "SELECT COUNT(*) FROM starter_authz_decisions WHERE surface = ?1 AND effect = 'deny'",
    )
    .bind(surface)
    .fetch_one(pool.sqlx())
    .await
    .expect("count");
    r.get::<i64, _>(0)
}

async fn wait_until_count(pool: &Pool, deadline: Duration, target: i64) {
    use sqlx::Row;
    let start = Instant::now();
    while start.elapsed() < deadline {
        let r = sqlx::query("SELECT COUNT(*) FROM starter_authz_decisions")
            .fetch_one(pool.sqlx())
            .await
            .expect("count");
        if r.get::<i64, _>(0) >= target {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

/// One deny issued with `with_surface("rest", …)` lands in
/// `starter_authz_decisions` with `surface = "rest"`. Symmetric
/// tests for "mcp" and "grpc" prove the dispatch path is the same
/// — the surface name is just a label.
#[tokio::test]
async fn rest_mcp_grpc_denies_share_audit_trail_distinguishably() {
    let pool = fresh_pool().await;
    let registry = Arc::new(StaticRegistry::new());
    registry.register(ResourceSpec::from_static(
        "weather",
        &["read", "refresh"],
        Ownership::None,
        "weather",
        "",
    ));
    let sink: Arc<dyn DecisionSink> = Arc::new(DbDecisionSink::sqlite(
        pool.clone(),
        DecisionSinkConfig::new(),
    ));
    let engine = Arc::new(
        StaticRbacEngine::from_config(
            AuthzConfig::default(),
            registry.clone() as Arc<dyn ResourceRegistry>,
        )
        .unwrap()
        .with_sink(sink),
    );

    let p = reader("alice");
    let object = ResourceRef::collection("weather");

    // One deny per surface (Reader cannot `refresh`). The engine
    // populates `entry.surface` from the task-local each adapter
    // would bind. Run sequentially so each surface label only
    // applies to its own check.
    let d = with_surface("rest", engine.check(&p, "refresh", &object)).await;
    assert!(matches!(
        d,
        starter_spi::authz::Decision::Deny { .. }
    ));
    let d = with_surface("mcp", engine.check(&p, "refresh", &object)).await;
    assert!(matches!(
        d,
        starter_spi::authz::Decision::Deny { .. }
    ));
    let d = with_surface("grpc", engine.check(&p, "refresh", &object)).await;
    assert!(matches!(
        d,
        starter_spi::authz::Decision::Deny { .. }
    ));

    // The sink is async (bounded channel + writer task), so wait
    // briefly for the rows to drain. Non-ordering-sensitive: we
    // count per surface label, not row order.
    wait_until_count(&pool, Duration::from_secs(2), 3).await;

    assert_eq!(
        count_surface(&pool, "rest").await,
        1,
        "exactly one REST deny row"
    );
    assert_eq!(
        count_surface(&pool, "mcp").await,
        1,
        "exactly one MCP deny row"
    );
    assert_eq!(
        count_surface(&pool, "grpc").await,
        1,
        "exactly one gRPC deny row"
    );
}

/// A check issued without [`with_surface`] lands with `surface =
/// NULL` (the engine's audit pipeline reads `None` from the
/// task-local). Proves the field is opt-in: pre-7d.2 in-process
/// callers (background jobs, tests) keep working unchanged.
#[tokio::test]
async fn check_without_surface_scope_leaves_column_null() {
    let pool = fresh_pool().await;
    let registry = Arc::new(StaticRegistry::new());
    registry.register(ResourceSpec::from_static(
        "weather",
        &["read", "refresh"],
        Ownership::None,
        "weather",
        "",
    ));
    let sink: Arc<dyn DecisionSink> = Arc::new(DbDecisionSink::sqlite(
        pool.clone(),
        DecisionSinkConfig::new(),
    ));
    let engine = Arc::new(
        StaticRbacEngine::from_config(
            AuthzConfig::default(),
            registry.clone() as Arc<dyn ResourceRegistry>,
        )
        .unwrap()
        .with_sink(sink),
    );

    let p = reader("alice");
    let object = ResourceRef::collection("weather");
    let _ = engine.check(&p, "refresh", &object).await;

    wait_until_count(&pool, Duration::from_secs(2), 1).await;

    use sqlx::Row;
    let r = sqlx::query("SELECT surface FROM starter_authz_decisions LIMIT 1")
        .fetch_one(pool.sqlx())
        .await
        .expect("row");
    let surface: Option<String> = r.get(0);
    assert!(
        surface.is_none(),
        "in-process check should leave surface NULL, got {surface:?}"
    );
}
