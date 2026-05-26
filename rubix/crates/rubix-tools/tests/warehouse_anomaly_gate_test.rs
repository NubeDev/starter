//! Integration test for the Stage 04 anomaly gate's dispatch hook.
//!
//! Mirrors `system_disk_insights_test.rs`: exercise the pure
//! `check_*` functions on synthetic fixtures, then hand each
//! returned diagnostic to `alert_send::dispatch` and assert the
//! process-wide counter moves by the expected delta. Asserting the
//! dispatch wiring this way (rather than spinning up the cleaner
//! verb against a live ClickHouse) keeps the test hermetic — the
//! verb's responsibility is purely to forward the gate's output.

use std::sync::Mutex;

use rubix_spi::dto::system::alert_send::AlertSeverity;
use rubix_tools::system::alert_send;
use rubix_tools::warehouse::anomaly_gate::{
    check_spike, check_stuck, CleanedRow, SpikeRow, STUCK_RUN_MIN,
};

/// Serialise dispatch-counter assertions: `ALERTS_FIRED` is
/// process-wide so two `#[tokio::test]`s reading the delta in
/// parallel race. The disk insights test escapes this by being
/// the only dispatch-firing test in its binary; this file
/// asserts three deltas so it locks instead.
static DISPATCH_LOCK: Mutex<()> = Mutex::new(());

fn row(meter: &str, ts_ms: i64, value: Option<f64>, quality: &str) -> CleanedRow {
    CleanedRow {
        tenant_id: "site-a".to_owned(),
        meter_id: meter.to_owned(),
        bucket_start_ms: ts_ms,
        value,
        quality: quality.to_owned(),
    }
}

fn spike(meter: &str, ts_ms: i64, value: f64) -> SpikeRow {
    SpikeRow {
        tenant_id: "site-a".to_owned(),
        meter_id: meter.to_owned(),
        epoch_ms: ts_ms,
        value,
    }
}

#[tokio::test]
async fn spike_fixture_dispatches_one_alert_per_suspect_row() {
    let _guard = DISPATCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let rows = vec![spike("site-a.elec.main", 60_000, 9999.0)];
    let before = alert_send::dispatched_count();
    let diags = check_spike(&rows);
    assert_eq!(diags.len(), 1, "fixture carries one suspect row");
    for d in diags {
        alert_send::dispatch(AlertSeverity::Warn, d)
            .await
            .expect("alert_send::dispatch must succeed for a well-formed diagnostic");
    }
    let after = alert_send::dispatched_count();
    assert_eq!(
        after - before,
        1,
        "R-SPIKE must dispatch exactly one rubix.alert.send per L1 suspect row",
    );
}

#[tokio::test]
async fn stuck_fixture_dispatches_one_alert_per_meter() {
    let _guard = DISPATCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bucket = 60_000i64;
    let rows: Vec<CleanedRow> = (0..(STUCK_RUN_MIN as i64 + 1))
        .map(|i| row("site-a.water.main", bucket + i * bucket, Some(7.0), "ok"))
        .collect();
    let before = alert_send::dispatched_count();
    let diags = check_stuck(&rows);
    assert_eq!(
        diags.len(),
        1,
        "one meter, one ≥{STUCK_RUN_MIN}-bucket run, one diagnostic",
    );
    for d in diags {
        alert_send::dispatch(AlertSeverity::Error, d)
            .await
            .expect("alert_send::dispatch must succeed for a well-formed diagnostic");
    }
    let after = alert_send::dispatched_count();
    assert_eq!(
        after - before,
        1,
        "R-STUCK must dispatch exactly one rubix.alert.send per stuck meter",
    );
}

#[tokio::test]
async fn benign_fixture_dispatches_nothing() {
    let _guard = DISPATCH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bucket = 60_000i64;
    let cleaned: Vec<CleanedRow> = (0..10)
        .map(|i| row("site-a.elec.hvac", bucket + i * bucket, Some(i as f64), "ok"))
        .collect();
    let before = alert_send::dispatched_count();
    for d in check_spike(&[]) {
        alert_send::dispatch(AlertSeverity::Warn, d).await.unwrap();
    }
    for d in check_stuck(&cleaned) {
        alert_send::dispatch(AlertSeverity::Error, d).await.unwrap();
    }
    let after = alert_send::dispatched_count();
    assert_eq!(
        after - before,
        0,
        "benign fixture (no spike rows, all distinct cleaned values) must dispatch nothing",
    );
}

