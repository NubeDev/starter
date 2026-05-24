//! Integration tests for [`service::FlowAsService::register_schedule`]
//! and [`service::FlowAsService::unregister_schedule`] against a
//! real Postgres via testcontainers.
//!
//! Marked `#[ignore]` because they require Docker — the suite is
//! invoked via `cargo test -p starter-flow-surfaces --
//! --ignored`.

#![cfg(test)]

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use sqlx::postgres::PgListener;
use uuid::Uuid;

use starter_flow_surfaces::clock::{Clock, TestClock};
use starter_flow_surfaces::service::{FlowAsService, FlowRunner};
use starter_flow_surfaces::FlowRegistry;
use starter_store_postgres::{
    migrate, testing::with_database, SCHEDULED_FLOWS_MIGRATION_SOURCE,
};

const CHANNEL: &str = "starter_scheduled_flows";
const FLOW_ID: &str = "com.rubix.weekly-report";
const CRON_WEEKLY: &str = "0 0 8 * * MON"; // Monday 08:00 UTC (6-field).

/// Test-only [`FlowRunner`] that never gets called this stage —
/// the register/unregister surface doesn't dispatch flows. The
/// scaffold needs *some* runner to construct the service; this
/// one panics if invoked so a future regression that quietly
/// starts dispatching from `register_schedule` is loud.
struct PanicRunner;

#[async_trait]
impl FlowRunner for PanicRunner {
    async fn run(
        &self,
        _tenant_id: Uuid,
        _flow_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        panic!("Phase B.1 must not dispatch through FlowRunner");
    }
}

fn make_service(pool: starter_store_postgres::Pool, clock: TestClock) -> FlowAsService {
    let registry = Arc::new(FlowRegistry::new());
    let runner: Arc<dyn FlowRunner> = Arc::new(PanicRunner);
    FlowAsService::new(pool, registry, runner).with_clock(Arc::new(clock) as Arc<dyn Clock>)
}

#[tokio::test]
#[ignore = "requires docker"]
async fn register_schedule_inserts_row_and_emits_notify() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    let mut listener = PgListener::connect_with(pool.sqlx())
        .await
        .expect("PgListener connect");
    listener.listen(CHANNEL).await.expect("LISTEN");

    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap());
    let svc = make_service(pool.clone(), clock.clone());
    let tenant = Uuid::nil();

    let next = svc
        .register_schedule(tenant, FLOW_ID, CRON_WEEKLY)
        .await
        .expect("register succeeds");

    // The clock seed is a Sunday at 00:00; next Monday-08:00 is
    // exactly 32 hours later.
    assert!(next > clock.now(), "next_run_at must be in the future");
    assert_eq!(next.timestamp() - clock.now().timestamp(), 32 * 3600);

    // Notify fires on insert.
    let notif = tokio::time::timeout(Duration::from_secs(10), listener.recv())
        .await
        .expect("notify in time")
        .expect("notify present");
    assert_eq!(notif.channel(), CHANNEL);
    let payload: serde_json::Value =
        serde_json::from_str(notif.payload()).expect("payload is json");
    assert_eq!(payload["op"], "INSERT");
    assert_eq!(payload["flow_id"], FLOW_ID);
    assert_eq!(payload["enabled"], true);

    // The row is readable via the lookup helper.
    let row = svc
        .lookup_schedule(tenant, FLOW_ID)
        .await
        .expect("lookup succeeds")
        .expect("row exists");
    assert_eq!(row.0, next);
    assert!(row.1, "enabled must be TRUE on fresh register");
}

#[tokio::test]
#[ignore = "requires docker"]
async fn register_schedule_is_idempotent_via_on_conflict() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap());
    let svc = make_service(pool.clone(), clock.clone());
    let tenant = Uuid::nil();

    let first = svc
        .register_schedule(tenant, FLOW_ID, CRON_WEEKLY)
        .await
        .expect("first register");

    // Advance the clock past the original next_run_at, then
    // re-register with a different expression — the row is
    // updated in-place, the new next_run_at reflects the new
    // expression against the new now.
    clock.advance(chrono::Duration::days(2));
    let cron_every_minute = "0 * * * * *";
    let second = svc
        .register_schedule(tenant, FLOW_ID, cron_every_minute)
        .await
        .expect("second register");

    assert!(second > first || second != first, "next_run_at must change");
    let row = svc
        .lookup_schedule(tenant, FLOW_ID)
        .await
        .expect("lookup")
        .expect("row");
    assert_eq!(row.0, second);
    assert!(row.1);
}

#[tokio::test]
#[ignore = "requires docker"]
async fn register_schedule_rejects_invalid_cron() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    let svc = make_service(pool.clone(), TestClock::epoch());
    let err = svc
        .register_schedule(Uuid::nil(), FLOW_ID, "definitely not cron")
        .await
        .expect_err("invalid cron must fail before touching PG");
    let msg = format!("{err}");
    assert!(
        msg.contains("invalid cron expression"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
#[ignore = "requires docker"]
async fn unregister_schedule_flips_enabled_and_notifies() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    let clock = TestClock::new(Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap());
    let svc = make_service(pool.clone(), clock);
    let tenant = Uuid::nil();

    svc.register_schedule(tenant, FLOW_ID, CRON_WEEKLY)
        .await
        .expect("register");

    // Subscribe AFTER the insert so we only see the unregister
    // notify in the assertion below — keeps the test pinned to
    // one event without timing-dependent drains.
    let mut listener = PgListener::connect_with(pool.sqlx())
        .await
        .expect("PgListener connect");
    listener.listen(CHANNEL).await.expect("LISTEN");

    let affected = svc
        .unregister_schedule(tenant, FLOW_ID)
        .await
        .expect("unregister");
    assert!(affected, "first unregister must report rows affected");

    let notif = tokio::time::timeout(Duration::from_secs(10), listener.recv())
        .await
        .expect("notify in time")
        .expect("notify present");
    let payload: serde_json::Value =
        serde_json::from_str(notif.payload()).expect("payload is json");
    assert_eq!(payload["op"], "UPDATE");
    assert_eq!(payload["enabled"], false);

    let row = svc
        .lookup_schedule(tenant, FLOW_ID)
        .await
        .expect("lookup")
        .expect("row still present (soft-delete)");
    assert!(!row.1, "enabled must be FALSE after unregister");

    // Idempotent: second unregister finds no `enabled = TRUE`
    // row and returns false without firing NOTIFY.
    let again = svc
        .unregister_schedule(tenant, FLOW_ID)
        .await
        .expect("idempotent unregister");
    assert!(!again, "second unregister reports no rows affected");
}
