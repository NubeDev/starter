//! Integration test for
//! [`service::FlowAsService::tick`](starter_flow_surfaces::service::FlowAsService::tick).
//!
//! Phase B.2 of Goal 6 (see
//! `.codeless/jobs/rubix-goal-6-weekly-report/SCOPE.md`). Pins
//! the durable-scheduler claim/dispatch/bookkeeping cycle against
//! a real Postgres provisioned via testcontainers.
//!
//! Strategy
//! --------
//!
//! - Register a single `(tenant, flow)` schedule with the
//!   every-minute cron `0 * * * * *`.
//! - Drive a deterministic [`TestClock`] forward by exactly two
//!   minutes, calling [`FlowAsService::tick`] after each
//!   one-minute advance.
//! - Assert: (a) two dispatches landed on the counting
//!   [`FlowRunner`] stub, (b) `last_run_at` / `last_run_status`
//!   are populated on the row after both ticks, and (c)
//!   `next_run_at` rolled forward each tick so the row never
//!   double-fires within the same wall-clock minute.
//!
//! Marked `#[ignore]` because it requires Docker — `cargo test
//! -p starter-flow-surfaces -- --ignored` is the suite entry.

#![cfg(test)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use sqlx::Row;
use uuid::Uuid;

use starter_flow_surfaces::clock::{Clock, TestClock};
use starter_flow_surfaces::service::{FlowAsService, FlowRunner};
use starter_flow_surfaces::FlowRegistry;
use starter_store_postgres::{
    migrate, testing::with_database, SCHEDULED_FLOWS_MIGRATION_SOURCE,
};

const FLOW_ID: &str = "com.rubix.weekly-report";
/// 6-field every-minute cron (`sec min hour dom mon dow`).
const CRON_EVERY_MINUTE: &str = "0 * * * * *";

/// Counts every dispatch so the test can assert "two fires."
struct CountingRunner {
    fires: Arc<AtomicUsize>,
}

#[async_trait]
impl FlowRunner for CountingRunner {
    async fn run(
        &self,
        _tenant_id: Uuid,
        _flow_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.fires.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn tick_fires_twice_when_clock_advances_two_minutes() {
    let (pool, _guard) = with_database().await;
    migrate(&pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    // Seed the clock at a tidy minute boundary so the first
    // `next_fire` lands exactly one minute later, not 59
    // seconds later (which would still work, but obscures the
    // assertion).
    let t0 = Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap();
    let clock = TestClock::new(t0);

    let fires = Arc::new(AtomicUsize::new(0));
    let runner: Arc<dyn FlowRunner> = Arc::new(CountingRunner {
        fires: fires.clone(),
    });
    let svc = FlowAsService::new(pool.clone(), Arc::new(FlowRegistry::new()), runner)
        .with_clock(Arc::new(clock.clone()) as Arc<dyn Clock>);

    let tenant = Uuid::nil();
    let first_next = svc
        .register_schedule(tenant, FLOW_ID, CRON_EVERY_MINUTE)
        .await
        .expect("register succeeds");

    // First firing is one minute after t0.
    assert_eq!(
        first_next.timestamp() - t0.timestamp(),
        60,
        "every-minute cron must place first fire at t0+60s"
    );

    // A tick run before any time advances must claim zero rows
    // (the row's `next_run_at` is in the future).
    let claimed = svc.tick().await.expect("pre-advance tick");
    assert_eq!(claimed, 0, "no rows due before the clock advances");
    assert_eq!(fires.load(Ordering::SeqCst), 0);

    // Minute 1: advance to t0+60s, tick — exactly one fire.
    clock.advance(ChronoDuration::seconds(60));
    let claimed = svc.tick().await.expect("first tick");
    assert_eq!(claimed, 1, "exactly one row due at t0+60s");
    assert_eq!(fires.load(Ordering::SeqCst), 1, "runner fired once");

    // Minute 2: advance to t0+120s, tick — second fire.
    clock.advance(ChronoDuration::seconds(60));
    let claimed = svc.tick().await.expect("second tick");
    assert_eq!(claimed, 1, "exactly one row due at t0+120s");
    assert_eq!(fires.load(Ordering::SeqCst), 2, "runner fired twice");

    // Bookkeeping landed: status `succeeded`, `last_run_at`
    // tracks the most recent clock reading, `next_run_at`
    // rolled forward.
    let row = sqlx::query(
        r#"SELECT next_run_at, last_run_at, last_run_status, last_run_message, enabled
             FROM starter_scheduled_flows
            WHERE tenant_id = $1 AND flow_id = $2"#,
    )
    .bind(tenant)
    .bind(FLOW_ID)
    .fetch_one(pool.sqlx())
    .await
    .expect("row");

    let next_run_at: chrono::DateTime<Utc> = row.get("next_run_at");
    let last_run_at: Option<chrono::DateTime<Utc>> = row.get("last_run_at");
    let last_run_status: Option<String> = row.get("last_run_status");
    let last_run_message: Option<String> = row.get("last_run_message");
    let enabled: bool = row.get("enabled");

    assert!(enabled, "row stays enabled across fires");
    assert_eq!(last_run_status.as_deref(), Some("succeeded"));
    assert_eq!(last_run_message, None, "succeeded runs leave message NULL");
    let last = last_run_at.expect("last_run_at populated");
    assert_eq!(last, clock.now(), "last_run_at tracks the tick clock");
    assert!(
        next_run_at > clock.now(),
        "next_run_at must be re-armed in the future, was {next_run_at} vs now {}",
        clock.now()
    );
    // The next firing relative to "now == t0+120s" is t0+180s.
    assert_eq!(
        next_run_at.timestamp() - clock.now().timestamp(),
        60,
        "post-tick next_run_at is one minute out"
    );

    // A third tick at the same clock must claim zero (idempotency
    // — the previous tick already advanced `next_run_at`).
    let claimed = svc.tick().await.expect("third tick");
    assert_eq!(claimed, 0, "no rows due until clock advances again");
    assert_eq!(fires.load(Ordering::SeqCst), 2, "no extra fires");
}
