//! Goal 6 (weekly-report) end-to-end integration coverage.
//!
//! Pins the full scheduled-flow loop the rubix-agent boot wires
//! at start: a [`FlowAsService`] over testcontainers Postgres
//! claims a due `starter_scheduled_flows` row, dispatches it
//! through a [`FlowRunner`] that runs `rubix.analytics.report`
//! against testcontainers ClickHouse + a tempdir-backed
//! `FsBlobStore`, the rendered HTML blob lands with the expected
//! per-day rows from the `disk_history_weekly` template, and
//! firing `rubix.undo.last` afterwards calls
//! [`AnalyticsReportReversible`] which deletes the blob.
//!
//! The test stands in for the production boot wiring of
//! [`rubix_agent::boot::scheduler::spawn`] — same
//! [`FlowAsService`] + [`Clock`] + [`FlowRunner`] composition,
//! same [`UndoDispatcher`] +
//! [`AnalyticsReportReversible`] composition the agent will use
//! once `build_tool_registry` learns the analytics verbs. The
//! seams are unchanged; only the runner is a test stub
//! (`AnalyticsReportRunner`) that bypasses the ai-agent loop so
//! this test does not need a live or fixture LLM.
//!
//! Marked `#[ignore]` because it needs Docker — run with
//! `cargo test -p rubix-agent --test goal_6_weekly_report_test -- --ignored`.

use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{Duration as ChronoDuration, TimeZone, Utc};
use futures::TryStreamExt;
use serde_json::{json, Value};
use sqlx::Row;
use tokio::sync::Mutex as TokioMutex;
use uuid::Uuid;

use rubix_spi::dto::analytics::report::{AnalyticsReportResponse, ReportFormat};
use rubix_tools::analytics::report::{AnalyticsReportReversible, AnalyticsReportTool};
use rubix_tools::undo::dispatch::{StaticActor, UndoDispatcher};
use rubix_tools::undo::last::UndoLastTool;

use starter_blob_fs::{FsBlobStore, PresignKey};
use starter_changelog::ChangeLog;
use starter_changelog_sqlite::{
    migration_source as changelog_migration_source, SqliteChangeLog, SqliteChangeRecorder,
};
use starter_flow_surfaces::clock::{Clock, TestClock};
use starter_flow_surfaces::service::{FlowAsService, FlowRunner};
use starter_flow_surfaces::FlowRegistry;
use starter_spi::blob::{BlobRef, BlobRefInternal, BlobStore, Etag};
use starter_spi::changelog::Actor;
use starter_spi::tool::Tool;
use starter_store_clickhouse::testing::with_clickhouse;
use starter_store_clickhouse::ChClient;
use starter_store_postgres::{
    migrate, testing::with_database, SCHEDULED_FLOWS_MIGRATION_SOURCE,
};
use starter_store_sqlite::{migrate as sqlite_migrate, testing::ephemeral as sqlite_ephemeral};
use starter_undo::{ReversibleRegistry, UndoService};

const FLOW_ID: &str = "com.rubix.weekly-report";
/// 6-field weekly cron (`sec min hour dom mon dow`). Mondays
/// 08:00 UTC — the production yaml's intent expressed in the
/// 6-field grammar the `cron` crate (and therefore `starter-cron`)
/// expects. Pinning the test on this rather than parsing the
/// 5-field yaml keeps the assertion robust to future grammar
/// upgrades in `starter-cron`.
const CRON_WEEKLY_MON_0800: &str = "0 0 8 * * MON";

async fn ch_exec(client: &ChClient, sql: &str) {
    client
        .inner()
        .query(sql)
        .execute()
        .await
        .unwrap_or_else(|e| panic!("setup SQL failed: {e}\nSQL: {sql}"));
}

/// Seed `system_disk_history` with one row per day for the last
/// seven days. `disk_history_weekly` groups by day and orders
/// chronologically, so the rendered html table is expected to
/// contain exactly seven rows with `peak_percent` values 60..=66.
async fn seed_seven_days(client: &ChClient) {
    ch_exec(
        client,
        "CREATE TABLE IF NOT EXISTS system_disk_history (\
            tenant_id UUID, host String, percent_used UInt8, \
            free_bytes UInt64, epoch_ms Int64\
         ) ENGINE = MergeTree ORDER BY epoch_ms",
    )
    .await;
    let now = Utc::now();
    for offset in 0..7i64 {
        // One row per day; vary `percent_used` so the html table
        // contains a recognisable monotonic pattern (60..=66).
        let day = now - ChronoDuration::days(offset);
        let percent = 60u8 + offset as u8;
        let epoch_ms = day.timestamp_millis();
        ch_exec(
            client,
            &format!(
                "INSERT INTO system_disk_history VALUES \
                 (toUUID('00000000-0000-0000-0000-000000000000'),'h',{percent},1000,{epoch_ms})"
            ),
        )
        .await;
    }
}

async fn drain(store: &FsBlobStore, blob_id: &str) -> Vec<u8> {
    let r = BlobRef::mint(
        store.backend_id().clone(),
        blob_id.to_owned(),
        Etag::new(""),
        0,
    );
    let chunks: Vec<Bytes> = store
        .get(&r, None)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    chunks.iter().flat_map(|b| b.iter().copied()).collect()
}

/// Returns `true` when the blob locator still resolves to bytes
/// on the FsBlobStore. The undo path is asserted by polling this
/// to `false` after `rubix.undo.last`.
async fn blob_exists(store: &FsBlobStore, blob_id: &str) -> bool {
    let r = BlobRef::mint(
        store.backend_id().clone(),
        blob_id.to_owned(),
        Etag::new(""),
        0,
    );
    store.get(&r, None).await.is_ok()
}

/// [`FlowRunner`] stub that stands in for the production
/// `ToolRegistryRunner` (see
/// `rubix-agent/src/boot/scheduler.rs`). Dispatches the
/// `analytics.report` verb directly through the undo-aware
/// [`UndoDispatcher`] — the same seam the ai-agent loop drives
/// once `build_tool_registry` exposes the analytics verbs. Records
/// the response so the test can assert on it after the tick
/// fires.
struct AnalyticsReportRunner {
    dispatcher: Arc<UndoDispatcher<AnalyticsReportTool>>,
    last_response: Arc<TokioMutex<Option<AnalyticsReportResponse>>>,
    last_reply: Arc<TokioMutex<Option<String>>>,
}

#[async_trait]
impl FlowRunner for AnalyticsReportRunner {
    async fn run(
        &self,
        _tenant_id: Uuid,
        flow_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert_eq!(
            flow_id, FLOW_ID,
            "test runner only knows the weekly-report flow",
        );
        let out: Value = self
            .dispatcher
            .invoke(json!({
                "template": "weekly-ops",
                "queries":  ["disk_history_weekly"],
                "format":   "html"
            }))
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::<dyn std::error::Error + Send + Sync>::from(format!("dispatch: {e}"))
            })?;
        let resp: AnalyticsReportResponse =
            serde_json::from_value(out).map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                Box::<dyn std::error::Error + Send + Sync>::from(format!("decode: {e}"))
            })?;
        // Stand-in for the ai-agent's terminal reply — production
        // wires this to the model's `text` turn after the
        // `analytics.report` tool_use, which the bundled skill
        // primes to summarise the rendered report.
        let reply = format!(
            "Weekly report rendered ({byte_count} bytes, {fmt:?}).",
            byte_count = resp.byte_count,
            fmt = resp.format,
        );
        *self.last_response.lock().await = Some(resp);
        *self.last_reply.lock().await = Some(reply);
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires docker"]
async fn weekly_report_fires_via_scheduler_renders_blob_and_undo_deletes_it() {
    // ----- 1. Backing stores --------------------------------------------
    // Postgres for `starter_scheduled_flows`.
    let (pg_pool, _pg_guard) = with_database().await;
    migrate(&pg_pool)
        .with_source(SCHEDULED_FLOWS_MIGRATION_SOURCE)
        .run()
        .await
        .expect("scheduled_flows migration applies");

    // SQLite for the changelog — the same swap goal_3 + goal_4
    // tests use. The recorder/log trait shapes are identical to
    // the Postgres impl, so this is a cost optimisation, not a
    // contract change.
    let sql_pool = sqlite_ephemeral().await;
    sqlite_migrate(&sql_pool)
        .with_source(changelog_migration_source())
        .run()
        .await
        .expect("apply changelog migration");
    let recorder = Arc::new(SqliteChangeRecorder::new(sql_pool.clone()));
    let log: Arc<dyn ChangeLog> = Arc::new(SqliteChangeLog::new(sql_pool.clone()));

    // ClickHouse for the analytics warehouse.
    let (ch_client, _ch_guard) = with_clickhouse().await;
    seed_seven_days(&ch_client).await;

    // Tempdir-backed FsBlobStore — same backend
    // `rubix.analytics.report` integration test uses, same
    // backend production wires via `starter-blob-fs`.
    let tempdir = tempfile::tempdir().expect("tempdir");
    let blob_store = FsBlobStore::open(tempdir.path(), PresignKey::ephemeral())
        .expect("FsBlobStore::open");
    let blob_arc: Arc<dyn BlobStore> = Arc::new(blob_store.clone());

    // ----- 2. Tool wiring (analytics.report + undo.last) ----------------
    let report_tool = Arc::new(AnalyticsReportTool::new(
        Arc::new(ch_client),
        blob_arc.clone(),
    ));
    let reversible = Arc::new(AnalyticsReportReversible::new(blob_arc.clone()));
    let registry = Arc::new(ReversibleRegistry::new().insert(reversible));

    let actor = Actor::User {
        subject: "scheduler@rubix".into(),
    };
    let actor_source = Arc::new(StaticActor(actor.clone()));

    let dispatcher = Arc::new(UndoDispatcher::new(
        report_tool,
        registry.clone(),
        recorder.clone(),
        actor_source.clone(),
    ));

    let undo_service = Arc::new(UndoService::new(log.clone(), registry.clone()));
    let undo_last = UndoLastTool::new(undo_service, actor_source);

    // ----- 3. FlowAsService + TestClock + stub runner -------------------
    let last_response: Arc<TokioMutex<Option<AnalyticsReportResponse>>> =
        Arc::new(TokioMutex::new(None));
    let last_reply: Arc<TokioMutex<Option<String>>> = Arc::new(TokioMutex::new(None));
    let runner: Arc<dyn FlowRunner> = Arc::new(AnalyticsReportRunner {
        dispatcher: dispatcher.clone(),
        last_response: last_response.clone(),
        last_reply: last_reply.clone(),
    });

    // Anchor the clock at a Sunday so the next Monday-08:00
    // firing lands within the 7-day advance window the SCOPE
    // pins.
    let t0 = Utc.with_ymd_and_hms(2026, 5, 24, 0, 0, 0).unwrap(); // Sun
    let clock = TestClock::new(t0);
    let svc = FlowAsService::new(
        pg_pool.clone(),
        Arc::new(FlowRegistry::new()),
        runner,
    )
    .with_clock(Arc::new(clock.clone()) as Arc<dyn Clock>);

    let tenant = Uuid::nil();
    let first_next = svc
        .register_schedule(tenant, FLOW_ID, CRON_WEEKLY_MON_0800)
        .await
        .expect("register_schedule succeeds");
    // The first fire is on the following Monday 08:00 UTC =
    // 2026-05-25 08:00 = t0 + 32h. Sanity-check the math so a
    // future cron-grammar regression surfaces here, not in a
    // mysterious "tick did not fire" assertion.
    assert_eq!(
        first_next,
        Utc.with_ymd_and_hms(2026, 5, 25, 8, 0, 0).unwrap(),
        "first fire must be next Monday 08:00 UTC",
    );

    // ----- 4. Advance clock by 7 days, tick, assert one fire ------------
    let claimed_pre = svc.tick().await.expect("pre-advance tick");
    assert_eq!(claimed_pre, 0, "no rows due before the clock advances");

    clock.advance(ChronoDuration::days(7));
    let claimed = svc.tick().await.expect("post-advance tick");
    assert_eq!(claimed, 1, "exactly one row due after a 7-day advance");

    // Bookkeeping: row lands as `succeeded`, `next_run_at`
    // re-armed in the future.
    let row = sqlx::query(
        "SELECT last_run_status, last_run_message, next_run_at, enabled \
           FROM starter_scheduled_flows WHERE tenant_id = $1 AND flow_id = $2",
    )
    .bind(tenant)
    .bind(FLOW_ID)
    .fetch_one(pg_pool.sqlx())
    .await
    .expect("row");
    let status: Option<String> = row.get("last_run_status");
    let message: Option<String> = row.get("last_run_message");
    let next_run_at: chrono::DateTime<Utc> = row.get("next_run_at");
    let enabled: bool = row.get("enabled");
    assert_eq!(status.as_deref(), Some("succeeded"), "run succeeded");
    assert_eq!(message, None, "succeeded runs leave message NULL");
    assert!(enabled, "row stays enabled");
    assert!(
        next_run_at > clock.now(),
        "next_run_at re-armed (got {next_run_at}, now {})",
        clock.now()
    );

    // ----- 5. The agent reply is non-empty and the report landed --------
    let reply = last_reply
        .lock()
        .await
        .clone()
        .expect("runner recorded a reply");
    assert!(!reply.trim().is_empty(), "agent reply is non-empty: {reply:?}");

    let resp = last_response
        .lock()
        .await
        .clone()
        .expect("runner recorded a response");
    assert_eq!(resp.summary.code.as_str(), "rubix.analytics.report.rendered");
    assert_eq!(resp.format, ReportFormat::Html);
    assert!(resp.byte_count > 0, "non-zero byte_count");
    assert!(!resp.url.is_empty(), "presigned url non-empty");
    assert!(
        resp.blob_id.starts_with("reports/weekly-ops/"),
        "blob_id namespace: {}",
        resp.blob_id
    );

    let blob_id = resp.blob_id.clone();
    assert!(
        blob_exists(&blob_store, &blob_id).await,
        "blob present in FsBlobStore after tick",
    );

    // The rendered html table must carry the seven seeded days.
    // The exporter emits headers `day` / `avg_percent` /
    // `peak_percent`; the seeded peaks span 60..=66 so each value
    // must appear in the table verbatim.
    let bytes = drain(&blob_store, &blob_id).await;
    let html = String::from_utf8(bytes).expect("rendered html is utf-8");
    assert!(
        html.contains("<h2>disk_history_weekly</h2>"),
        "html titles the query section: {html}"
    );
    assert!(html.contains("<th>day</th>"), "html includes day header");
    assert!(
        html.contains("<th>peak_percent</th>"),
        "html includes peak_percent header"
    );
    for peak in 60u8..=66 {
        assert!(
            html.contains(&format!(">{peak}<")) || html.contains(&format!(">{peak}.0<")),
            "html must include the seeded peak {peak}: {html}",
        );
    }

    // ----- 6. rubix.undo.last deletes the blob --------------------------
    let _ = undo_last
        .invoke(json!({}))
        .await
        .expect("undo.last dispatch succeeds");
    assert!(
        !blob_exists(&blob_store, &blob_id).await,
        "blob is gone after rubix.undo.last",
    );
}
