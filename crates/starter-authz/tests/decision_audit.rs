//! Phase 7c smoke tests — decision audit log. SCOPE-EXT.md R14.
//!
//! - `deny_eventually_recorded` — 100 denies + 2s deadline; the
//!   assertion is non-ordering-sensitive (count, not row order).
//! - `deny_drops_cleanly_on_overflow` — queue depth 4 + 1000
//!   denies + a writer that never drains; check that the server
//!   keeps serving and `dropped_count` is non-zero.
//! - `allow_sampled_at_rate` — sample=10 + 1000 allows + count in
//!   [80,120] (binomial spread).
//! - `audit_route_not_sampled` — the audit-log read kind opts out
//!   of sampling via the per-kind override map.
//! - `retention_task_deletes_expired` — `retention_pass_once`
//!   with a future cutoff wipes the table; with a past cutoff it
//!   leaves the table alone.

#![cfg(feature = "sqlite")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use starter_authz::audit::db::{retention_pass_once, should_sample_allow, DecisionFilter};
use starter_authz::audit::{DbDecisionSink, DecisionSinkConfig};
use starter_authz::store::AUTHZ_SQLITE_MIGRATOR;
use starter_authz::{
    AuthzConfig, DecisionSink, NoopDecisionSink, StaticRbacEngine, StaticRegistry,
};
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

fn registry(kind: &'static str, actions: &'static [&'static str]) -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    r.register(ResourceSpec::from_static(
        kind,
        actions,
        Ownership::None,
        kind,
        "",
    ));
    r
}

fn engine_with_sink(
    registry: Arc<dyn ResourceRegistry>,
    sink: Arc<dyn DecisionSink>,
) -> StaticRbacEngine {
    StaticRbacEngine::from_config(AuthzConfig::default(), registry)
        .unwrap()
        .with_sink(sink)
}

fn principal(subject: &str, role: Role) -> Principal {
    Principal {
        subject: subject.into(),
        role,
        scopes: vec![],
        tenant_id: None,
        teams: vec![],
        tenant_scope: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

async fn count_rows(pool: &Pool) -> i64 {
    use sqlx::Row;
    let r = sqlx::query("SELECT COUNT(*) FROM starter_authz_decisions")
        .fetch_one(pool.sqlx())
        .await
        .expect("count");
    r.get::<i64, _>(0)
}

async fn count_where(pool: &Pool, effect: &str) -> i64 {
    use sqlx::Row;
    let r = sqlx::query("SELECT COUNT(*) FROM starter_authz_decisions WHERE effect = ?1")
        .bind(effect)
        .fetch_one(pool.sqlx())
        .await
        .expect("count");
    r.get::<i64, _>(0)
}

/// Issue 100 denies. The default policy denies a Reader from
/// updating an unowned resource. Within 2s, all 100 rows are
/// persisted (non-ordering-sensitive — we count by effect).
#[tokio::test]
async fn deny_eventually_recorded() {
    let pool = fresh_pool().await;
    let sink = Arc::new(DbDecisionSink::sqlite(
        pool.clone(),
        DecisionSinkConfig::new(),
    ));
    let engine = engine_with_sink(registry("flows", &["update"]), sink.clone());

    let p = principal("u@x", Role::Reader);
    for _ in 0..100 {
        let d = engine
            .check(&p, "update", &ResourceRef::collection("flows"))
            .await;
        assert!(matches!(d, starter_spi::authz::Decision::Deny { .. }));
    }

    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let n = count_rows(&pool).await;
        if n >= 100 {
            break;
        }
        if Instant::now() > deadline {
            panic!("only {n} rows persisted within 2s; expected 100");
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let denies = count_where(&pool, "deny").await;
    assert_eq!(denies, 100, "every deny must persist (unsampled)");
}

/// Tiny queue + a writer task that gets starved by fast producers.
/// We don't artificially pause the writer here — we generate
/// requests faster than a single insert can drain on the same
/// runtime and assert that dropped_count is positive and the
/// engine keeps returning decisions (drop, don't block contract).
#[tokio::test]
async fn deny_drops_cleanly_on_overflow() {
    let pool = fresh_pool().await;
    let mut cfg = DecisionSinkConfig::new();
    cfg.queue_depth = 4;
    let sink = Arc::new(DbDecisionSink::sqlite(pool.clone(), cfg));
    let engine = engine_with_sink(registry("flows", &["update"]), sink.clone());

    let p = principal("u@x", Role::Reader);
    let start = Instant::now();
    for _ in 0..1000 {
        // The contract is "drop don't block" — every check must
        // return without awaiting a DB insert.
        let d = engine
            .check(&p, "update", &ResourceRef::collection("flows"))
            .await;
        assert!(matches!(d, starter_spi::authz::Decision::Deny { .. }));
    }
    let elapsed = start.elapsed();
    // Sanity: 1000 checks fit comfortably in a second even on a
    // slow CI box because the sink does try_send, not await an
    // insert.
    assert!(
        elapsed < Duration::from_secs(5),
        "check() blocked on sink: {elapsed:?}"
    );
    assert!(
        sink.dropped_count() > 0,
        "expected at least one row to drop on overflow (queue=4)"
    );
}

/// 1000 allows at sample=10 land between [80, 120] rows.
/// (Binomial spread for p=0.1 has mean 100, stddev ~9.5; the
/// ±20 window is ~2σ.)
#[tokio::test]
async fn allow_sampled_at_rate() {
    let pool = fresh_pool().await;
    let mut cfg = DecisionSinkConfig::new();
    cfg.allow_sample = 10;
    let sink = Arc::new(DbDecisionSink::sqlite(pool.clone(), cfg));
    let engine = engine_with_sink(registry("flows", &["read"]), sink.clone());

    // 1000 distinct subjects so the deterministic hash spreads.
    for i in 0..1000 {
        let p = principal(&format!("u{i}@x"), Role::Reader);
        let d = engine
            .check(&p, "read", &ResourceRef::collection("flows"))
            .await;
        assert!(matches!(d, starter_spi::authz::Decision::Allow { .. }));
    }

    // Wait for the writer task to drain. Allow up to 3s.
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let n = count_where(&pool, "allow").await;
        // Stop polling as soon as we plausibly have all the
        // sampled rows. We over-budget by waiting until the row
        // count stabilises across two consecutive polls.
        if n >= 60 || Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    let n = count_where(&pool, "allow").await;
    assert!(
        (80..=120).contains(&n),
        "allow count {n} outside [80, 120] for sample=10"
    );
}

/// `audit_logs` kind is force-sampled at N=1 by default — every
/// allow row for that kind persists, no matter the global sample.
#[tokio::test]
async fn audit_route_not_sampled() {
    let cfg = DecisionSinkConfig::new(); // default has audit_logs -> 1
                                         // Sample policy must always retain audit_logs reads.
    for i in 0..50 {
        let subject = format!("admin{i}@x");
        let keep = should_sample_allow(&cfg, "audit_logs", &subject, None);
        assert!(keep, "audit_logs allow must never be sampled away");
    }
    // Sanity: a different kind under the default N=100 sampling
    // does drop some allows (we don't pin the exact count here,
    // just that the sampler discriminates).
    let mut sampled = 0;
    let mut kept = 0;
    for i in 0..1000 {
        let subject = format!("u{i}@x");
        if should_sample_allow(&cfg, "flows", &subject, None) {
            kept += 1;
        } else {
            sampled += 1;
        }
    }
    assert!(sampled > 0, "default 1-in-100 should drop some allows");
    assert!(kept > 0, "default 1-in-100 should keep some allows");
}

/// Retention pass with a future cutoff wipes the table; with the
/// epoch as cutoff it leaves rows alone.
#[tokio::test]
async fn retention_task_deletes_expired() {
    let pool = fresh_pool().await;
    let sink = Arc::new(DbDecisionSink::sqlite(
        pool.clone(),
        DecisionSinkConfig::new(),
    ));
    let engine = engine_with_sink(registry("flows", &["update"]), sink.clone());

    let p = principal("u@x", Role::Reader);
    for _ in 0..20 {
        let _ = engine
            .check(&p, "update", &ResourceRef::collection("flows"))
            .await;
    }
    // Drain.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if count_rows(&pool).await >= 20 || Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(count_rows(&pool).await >= 20);

    // Cutoff in the past — keeps everything.
    let kept = retention_pass_once(
        &sink,
        chrono::Utc::now() - chrono::Duration::days(1),
        10_000,
    )
    .await
    .expect("retention pass");
    assert_eq!(kept, 0);
    assert!(count_rows(&pool).await >= 20);

    // Cutoff in the future — wipes everything.
    let removed = retention_pass_once(
        &sink,
        chrono::Utc::now() + chrono::Duration::seconds(60),
        10_000,
    )
    .await
    .expect("retention pass");
    assert!(removed >= 20, "expected >=20 rows deleted, got {removed}");
    assert_eq!(count_rows(&pool).await, 0);
}

/// `NoopDecisionSink` is the documented default; zero overhead,
/// zero rows. Sanity check it really is silent.
#[tokio::test]
async fn noop_sink_writes_nothing() {
    let pool = fresh_pool().await;
    let engine = engine_with_sink(registry("flows", &["update"]), Arc::new(NoopDecisionSink));
    let p = principal("u@x", Role::Reader);
    for _ in 0..50 {
        let _ = engine
            .check(&p, "update", &ResourceRef::collection("flows"))
            .await;
    }
    assert_eq!(count_rows(&pool).await, 0);
}

/// Filter sanity — list_via_sink honours the basic predicates.
#[tokio::test]
async fn list_via_sink_filters() {
    let pool = fresh_pool().await;
    let sink = Arc::new(DbDecisionSink::sqlite(
        pool.clone(),
        DecisionSinkConfig::new(),
    ));
    let engine = engine_with_sink(registry("flows", &["update"]), sink.clone());

    for i in 0..10 {
        let p = principal(&format!("u{i}@x"), Role::Reader);
        let _ = engine
            .check(&p, "update", &ResourceRef::collection("flows"))
            .await;
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if count_rows(&pool).await >= 10 || Instant::now() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let rows = starter_authz::audit::db::list_via_sink(
        &sink,
        &DecisionFilter {
            effect: Some("deny".into()),
            limit: 100,
            ..Default::default()
        },
    )
    .await
    .expect("query");
    assert_eq!(rows.len(), 10);
    let one = starter_authz::audit::db::list_via_sink(
        &sink,
        &DecisionFilter {
            subject: Some("u3@x".into()),
            limit: 100,
            ..Default::default()
        },
    )
    .await
    .expect("query");
    assert_eq!(one.len(), 1);
    assert_eq!(one[0].subject, "u3@x");
}
