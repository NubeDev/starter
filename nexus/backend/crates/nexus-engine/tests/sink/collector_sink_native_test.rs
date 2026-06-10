//! Native `collector` sink: it absorbs batches into the reserved bounded buffer
//! and trips the truncation flag (and cancels the run token) on the first
//! over-cap batch.

use nexus_engine::arrow_json::json_carrier_batch;
use nexus_engine::core::Processor;
use nexus_engine::core::Sink;
use nexus_engine::processor::JsonToArrow;
use nexus_engine::sink::store;
use nexus_engine::sink::CollectorSink;
use nexus_engine::Caps;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

/// Shape `rows` into one typed batch through `json_to_arrow`.
async fn typed(rows: &[Value]) -> datafusion::arrow::array::RecordBatch {
    let docs: Vec<String> = rows.iter().map(|r| r.to_string()).collect();
    let mut json = JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).unwrap();
    json.process(json_carrier_batch(&docs))
        .await
        .unwrap()
        .pop()
        .unwrap()
}

#[tokio::test]
async fn absorbs_rows_within_caps() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    store::open(&run_id, Caps::unbounded(), token);

    let mut sink = CollectorSink::from_config(&json!({ "type": "collector", "run_id": run_id }))
        .expect("resolve reserved buffer");
    let batch = typed(&[json!({ "n": 1 }), json!({ "n": 2 })]).await;
    sink.write(&batch).await.expect("write");

    let drained = store::take(&run_id);
    assert_eq!(drained.rows.len(), 2);
    assert!(!drained.truncated);
}

#[tokio::test]
async fn over_cap_batch_trips_truncation_and_cancels() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    store::open(&run_id, Caps::rows(1), token.clone());

    let mut sink = CollectorSink::from_config(&json!({ "type": "collector", "run_id": run_id }))
        .expect("resolve reserved buffer");
    // A two-row batch breaches the one-row cap whole: it is dropped, the run is
    // flagged truncated, and the token is cancelled so the source stops.
    let batch = typed(&[json!({ "n": 1 }), json!({ "n": 2 })]).await;
    sink.write(&batch).await.expect("write");

    assert!(token.is_cancelled(), "a breached cap cancels the run token");
    let drained = store::take(&run_id);
    assert!(drained.truncated, "the over-cap batch flags truncation");
    assert!(drained.rows.is_empty(), "an over-cap batch is dropped whole");
}
