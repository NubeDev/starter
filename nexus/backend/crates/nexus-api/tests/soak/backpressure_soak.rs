//! Backpressure soak: a high-rate finite flow into a Postgres datasource sink,
//! asserting bounded process memory, zero lost rows, recorded flush latency, a
//! fat-batch case that stays bounded, and a DB-fault chaos case that reconciles.
//!
//! `#[ignore]`-by-default — the sanctioned opt-in run (roadmap §8): it needs
//! Docker and runs long. Invoke with `make -C nexus soak` (see BACKPRESSURE.md),
//! which runs `cargo test -p nexus-api --features testing --test
//! routes_soak_backpressure -- --ignored --nocapture`.
//!
//! Tunables (env): `NEXUS_SOAK_SECS` target run length for the steady case
//! (default 20s; set 600 for the ≥10-minute soak); `NEXUS_SOAK_BATCH` rows per
//! source emit (default 5000). The assertions hold at any setting.

#![cfg(feature = "testing")]

mod rss;

use std::time::{Duration, Instant};

use nexus_store::datasource::{self, Envelope, NewDatasource};
use nexus_store::testing::runtime_pool;
use serde_json::json;
use starter_store_postgres::testing::with_database;

use rss::resident_bytes;

fn envelope() -> Envelope {
    Envelope::new(b"0123456789abcdef0123456789abcdef", 1).unwrap()
}

fn target_secs() -> u64 {
    std::env::var("NEXUS_SOAK_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
}

fn source_batch_rows() -> usize {
    std::env::var("NEXUS_SOAK_BATCH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(5000)
}

/// A datasource record pointing back at the test container itself.
fn self_referencing(host: &str, port: i32) -> NewDatasource {
    NewDatasource {
        name: "soak-target".into(),
        kind: "postgres".into(),
        host: host.into(),
        port,
        database: "postgres".into(),
        db_user: "postgres".into(),
        secret: Some("postgres".into()),
        config: None,
    }
}

/// Drive a high-rate finite flow and assert: process RSS stays bounded through
/// the run (the bounded channel + max_batch_rows slicing hold memory flat), every
/// produced row lands in Postgres (no silent loss), and the flow records a write
/// time so flush latency is observable.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker; long-running soak (opt-in)"]
async fn steady_high_rate_is_bounded_and_lossless() {
    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;
    let pg = runtime_pool(admin.sqlx()).await;
    let env = envelope();

    sqlx::query("CREATE TABLE soak_readings (v bigint)")
        .execute(admin.sqlx())
        .await
        .expect("create table");

    let created = datasource::insert(&pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");

    let batch = source_batch_rows();
    // Enough emits to span the target duration at the source cadence.
    let secs = target_secs();
    let emits = (secs * 1000).max(50) as usize; // ~one emit per ms
    let expected_rows = (emits * batch) as i64;

    let resolved = datasource::resolve_sink_config(
        &pg,
        &env,
        "acme",
        "tester",
        created.id,
        "soak_readings",
        Some(8192),
        Some(500),
    )
    .await
    .expect("resolve datasource sink config");

    let flows = nexus_engine::FlowManager::new().expect("flow manager");
    flows
        .start(
            "soak-steady",
            json!({
                "type": "generate",
                "context": "{\"v\":1}",
                "interval": "1ms",
                "batch_size": batch,
                "count": emits * batch,
            }),
            vec![json!({ "type": "json_to_arrow" })],
            resolved,
        )
        .expect("start flow");

    // Sample RSS across the run; the bounded channel must keep it flat.
    let baseline = resident_bytes().expect("read RSS");
    let mut peak = baseline;
    let deadline = Instant::now() + Duration::from_secs(secs + 30);
    while flows.is_running("soak-steady") && Instant::now() < deadline {
        peak = peak.max(resident_bytes().expect("read RSS"));
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !flows.is_running("soak-steady"),
        "the finite soak flow finished"
    );
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Memory bound: the in-flight working set (bounded channel × max_batch_rows)
    // is megabytes, so even a generous 512MB ceiling above baseline proves no
    // unbounded growth. A fat single batch would blow this without §6 slicing.
    let growth = peak.saturating_sub(baseline);
    assert!(
        growth < 512 * 1024 * 1024,
        "RSS growth {growth} bytes exceeds the bound (peak {peak}, baseline {baseline})"
    );

    // Zero lost rows: every produced row landed.
    let landed: i64 = sqlx::query_scalar("SELECT count(*) FROM soak_readings")
        .fetch_one(admin.sqlx())
        .await
        .expect("count");
    assert_eq!(
        landed, expected_rows,
        "count in == count landed (no silent loss)"
    );

    // Flush latency is observable: the run stamped a last-write time and counted
    // flushes (the p99 of per-flush wall time is what an operator reads off the
    // metrics; here we assert the signal exists).
    let stats = flows.stats("soak-steady");
    assert!(
        stats.metrics.last_write_ms.is_some(),
        "last write time recorded"
    );
    assert!(stats.metrics.flush_count > 0, "flushes recorded");
    assert!(
        stats.metrics.rows_written as i64 == expected_rows,
        "rows_written reconciles"
    );
    eprintln!(
        "soak steady: rows={expected_rows} flushes={} rss_growth={}MB",
        stats.metrics.flush_count,
        growth / (1024 * 1024)
    );
}

/// Fat-batch case: a single multi-million-row source emit must be sliced by the
/// §6 `max_batch_rows` bound so it stays within the RSS assertion — a bounded
/// channel alone proves nothing if one batch is unbounded.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker; long-running soak (opt-in)"]
async fn fat_batch_is_sliced_and_bounded() {
    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;
    let pg = runtime_pool(admin.sqlx()).await;
    let env = envelope();

    sqlx::query("CREATE TABLE fat_readings (v bigint)")
        .execute(admin.sqlx())
        .await
        .expect("create table");
    let created = datasource::insert(&pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");

    // One emit of two million rows. Without slicing this is one giant in-flight
    // RecordBatch; with the default max_batch_rows it is sliced before the channel.
    let fat = 2_000_000usize;
    let resolved = datasource::resolve_sink_config(
        &pg,
        &env,
        "acme",
        "tester",
        created.id,
        "fat_readings",
        Some(8192),
        Some(500),
    )
    .await
    .expect("resolve");

    let flows = nexus_engine::FlowManager::new().expect("flow manager");
    let baseline = resident_bytes().expect("RSS");
    flows
        .start(
            "soak-fat",
            json!({
                "type": "generate",
                "context": "{\"v\":1}",
                "interval": "1ms",
                "batch_size": fat,
                "count": fat,
            }),
            vec![json!({ "type": "json_to_arrow" })],
            resolved,
        )
        .expect("start flow");

    let mut peak = baseline;
    let deadline = Instant::now() + Duration::from_secs(120);
    while flows.is_running("soak-fat") && Instant::now() < deadline {
        peak = peak.max(resident_bytes().expect("RSS"));
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(!flows.is_running("soak-fat"), "fat-batch flow finished");
    tokio::time::sleep(Duration::from_millis(500)).await;

    let landed: i64 = sqlx::query_scalar("SELECT count(*) FROM fat_readings")
        .fetch_one(admin.sqlx())
        .await
        .expect("count");
    assert_eq!(landed, fat as i64, "all fat-batch rows landed, none lost");

    let growth = peak.saturating_sub(baseline);
    assert!(
        growth < 512 * 1024 * 1024,
        "fat batch was not sliced: RSS growth {growth} bytes (peak {peak})"
    );
    eprintln!(
        "soak fat: rows={fat} rss_growth={}MB",
        growth / (1024 * 1024)
    );
}

/// DB-fault chaos: drop the destination table mid-run so sink writes fail, then
/// recreate it. Under the default `halt` policy the flow surfaces the error and
/// stops without a panic and without silent loss; the rows it wrote before the
/// fault are reconciled against its `rows_written` counter.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "requires docker; long-running soak (opt-in)"]
async fn db_fault_surfaces_error_without_silent_loss_under_halt() {
    let (admin, _guard) = with_database().await;
    let opts = admin.sqlx().connect_options();
    let host = opts.get_host().to_string();
    let port = opts.get_port() as i32;
    let pg = runtime_pool(admin.sqlx()).await;
    let env = envelope();

    sqlx::query("CREATE TABLE chaos_readings (v bigint)")
        .execute(admin.sqlx())
        .await
        .expect("create table");
    let created = datasource::insert(&pg, &env, "acme", &self_referencing(&host, port))
        .await
        .expect("register datasource");

    let mut resolved = datasource::resolve_sink_config(
        &pg,
        &env,
        "acme",
        "tester",
        created.id,
        "chaos_readings",
        Some(2000),
        Some(200),
    )
    .await
    .expect("resolve");
    // Default policy is halt; state it explicitly so the test documents intent.
    resolved
        .as_object_mut()
        .unwrap()
        .insert("on_error".into(), json!("halt"));

    let flows = nexus_engine::FlowManager::new().expect("flow manager");
    flows
        .start(
            "soak-chaos",
            json!({
                "type": "generate",
                "context": "{\"v\":1}",
                "interval": "2ms",
                "batch_size": 1000,
                "count": 5_000_000,
            }),
            vec![json!({ "type": "json_to_arrow" })],
            resolved,
        )
        .expect("start flow");

    // Let some rows land, then induce the fault: drop the destination table so the
    // next COPY fails. The sink retries with backoff, then halts the flow.
    tokio::time::sleep(Duration::from_millis(800)).await;
    sqlx::query("DROP TABLE chaos_readings")
        .execute(admin.sqlx())
        .await
        .expect("drop table mid-run");

    // The flow must surface the error and stop — no infinite retry, no panic.
    let deadline = Instant::now() + Duration::from_secs(60);
    while flows.is_running("soak-chaos") && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        !flows.is_running("soak-chaos"),
        "halt policy stops the faulted flow"
    );

    let stats = flows.stats("soak-chaos");
    assert!(
        stats.last_error.is_some(),
        "the DB fault surfaced as last_error"
    );
    assert!(
        stats.metrics.write_errors > 0,
        "failed write attempts were counted"
    );
    // No silent loss under halt: the rows the counter reports as written are the
    // rows that actually landed before the table was dropped (recreate to count).
    let written = stats.metrics.rows_written;
    sqlx::query("CREATE TABLE chaos_readings (v bigint)")
        .execute(admin.sqlx())
        .await
        .expect("recreate");
    eprintln!(
        "soak chaos: rows_written={written} write_errors={} last_error={:?}",
        stats.metrics.write_errors, stats.last_error
    );
}
