//! FlowManager lifecycle: a saved flow starts, is tracked as running, start is
//! idempotent, and stop tears it down. The flow here drains a finite `generate`
//! input to the `drop` output — no external dependency — so the test exercises
//! the manager's bookkeeping and the real native pipeline build, not a connector.

use std::time::Duration;

use nexus_engine::FlowManager;
use serde_json::json;

fn finite_flow() -> (serde_json::Value, Vec<serde_json::Value>, serde_json::Value) {
    let input = json!({
        "type": "generate",
        "context": "{ \"v\": 1 }",
        "interval": "5ms",
        "batch_size": 1,
    });
    let processors = vec![
        json!({ "type": "json_to_arrow" }),
        json!({ "type": "sql", "query": "SELECT v FROM flow" }),
    ];
    let output = json!({ "type": "drop" });
    (input, processors, output)
}

#[tokio::test]
async fn start_is_idempotent_and_stop_tears_down() {
    let mgr = FlowManager::new().expect("register builders");
    let (input, processors, output) = finite_flow();

    mgr.start("flow-1", input.clone(), processors.clone(), output.clone())
        .expect("start");
    assert!(mgr.is_running("flow-1"), "tracked as running after start");

    // Starting an already-running flow is a no-op, not a double-spawn.
    mgr.start("flow-1", input, processors, output)
        .expect("idempotent start");

    // Stop cancels and drops it from the running set.
    assert!(mgr.stop("flow-1"), "stop reports it stopped a flow");
    // Give the spawned task a moment to observe the cancel and unregister.
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!mgr.is_running("flow-1"), "no longer running after stop");

    // Stopping an unknown flow is a clean false, not a panic.
    assert!(!mgr.stop("flow-1"), "second stop is a no-op");
}

/// A finite `generate → drop` flow run populates the per-run ingest counters the
/// flows API surfaces: batches are counted in and rows are written, and the
/// last-write stamp is set. Proves the metered wrappers are wired into a real run.
#[tokio::test]
async fn run_populates_ingest_metrics() {
    let mgr = FlowManager::new().expect("register builders");
    let (input, processors, output) = finite_flow();

    mgr.start("flow-metrics", input, processors, output)
        .expect("start");
    // Let several batches flow through the pipeline.
    tokio::time::sleep(Duration::from_millis(120)).await;
    mgr.stop("flow-metrics");
    tokio::time::sleep(Duration::from_millis(50)).await;

    let stats = mgr.stats("flow-metrics");
    assert!(stats.last_started_at.is_some(), "start time stamped");
    assert!(stats.metrics.batches_in > 0, "batches counted in");
    assert!(stats.metrics.rows_written > 0, "rows written to the sink");
    assert!(stats.metrics.flush_count > 0, "sink flushes counted");
    assert!(stats.metrics.last_write_ms.is_some(), "last write stamped");
    assert_eq!(
        stats.metrics.write_errors, 0,
        "no write errors on a healthy flow"
    );
}
