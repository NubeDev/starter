//! The `http_ingest` push source: a running flow registers a channel keyed by
//! its flow id; a push through the manager's ingest registry lands rows in the
//! sink. A push to an unknown flow is `NotRunning`; a push that overruns a tiny
//! channel is `Full` — the backpressure signal the REST route maps to `429`.

use std::time::Duration;

use nexus_engine::{FlowManager, IngestError};
use serde_json::json;

/// An `http_ingest → json_to_arrow → sql → drop` flow with a one-deep channel so
/// the backpressure path is easy to provoke. The SQL keeps a `v` column so the
/// metered sink counts a written row per accepted document.
fn push_flow(capacity: usize) -> (serde_json::Value, Vec<serde_json::Value>, serde_json::Value) {
    let input = json!({ "type": "http_ingest", "capacity": capacity });
    let processors = vec![
        json!({ "type": "json_to_arrow" }),
        json!({ "type": "sql", "query": "SELECT v FROM flow" }),
    ];
    let output = json!({ "type": "drop" });
    (input, processors, output)
}

#[tokio::test]
async fn push_to_running_flow_lands_rows() {
    let mgr = FlowManager::new().expect("register builders");
    let (input, processors, output) = push_flow(64);
    mgr.start("ingest-ok", input, processors, output)
        .expect("start");
    // The source registers its channel as the run spins up.
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        mgr.ingest().is_open("ingest-ok"),
        "channel open while running"
    );

    for n in 0..3 {
        mgr.ingest()
            .try_push("ingest-ok", vec![json!({ "v": n }).to_string()])
            .expect("push accepted onto an open channel");
    }
    tokio::time::sleep(Duration::from_millis(80)).await;

    let stats = mgr.stats("ingest-ok");
    assert!(
        stats.metrics.rows_written >= 3,
        "pushed rows reach the sink"
    );

    mgr.stop("ingest-ok");
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !mgr.ingest().is_open("ingest-ok"),
        "channel closes when the flow stops"
    );
}

#[tokio::test]
async fn push_to_unknown_flow_is_not_running() {
    let mgr = FlowManager::new().expect("register builders");
    let err = mgr
        .ingest()
        .try_push("nope", vec![json!({ "v": 1 }).to_string()])
        .expect_err("a push to an unregistered flow is rejected");
    assert_eq!(err, IngestError::NotRunning);
}

#[tokio::test]
async fn full_channel_reports_backpressure() {
    let mgr = FlowManager::new().expect("register builders");
    let (input, processors, output) = push_flow(1);
    mgr.start("ingest-full", input, processors, output)
        .expect("start");
    tokio::time::sleep(Duration::from_millis(20)).await;

    // `try_push` is synchronous and this loop never awaits, so the consumer task
    // cannot advance between pushes: with a 1-deep channel the burst overruns the
    // single slot deterministically and the source surfaces backpressure.
    let mut saw_full = false;
    for n in 0..512 {
        match mgr
            .ingest()
            .try_push("ingest-full", vec![json!({ "v": n }).to_string()])
        {
            Ok(()) => {}
            Err(IngestError::Full { retry_after_secs }) => {
                assert!(retry_after_secs >= 1, "retry-after hint is positive");
                saw_full = true;
                break;
            }
            Err(other) => panic!("unexpected push error: {other:?}"),
        }
    }
    assert!(
        saw_full,
        "a tight burst over a 1-deep channel hits backpressure"
    );

    mgr.stop("ingest-full");
}
