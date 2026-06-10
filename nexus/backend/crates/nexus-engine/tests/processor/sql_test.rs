//! `sql` processor parity: a SELECT over the `flow` table shapes and orders the
//! batch exactly as the recorded fixture from the query seam expects, an empty
//! batch is a no-op, and DML is rejected.

use nexus_engine::arrow_json::{batch_to_rows, json_carrier_batch};
use nexus_engine::core::Processor;
use nexus_engine::processor::{JsonToArrow, SqlProcessor};
use serde_json::json;

/// Parse two known rows into a typed batch the SQL processor can query — the
/// same `json_to_arrow → sql` path the query seam runs.
async fn typed_batch() -> datafusion::arrow::array::RecordBatch {
    let docs = [
        json!({ "city": "madrid", "temp_c": 33 }).to_string(),
        json!({ "city": "berlin", "temp_c": 21 }).to_string(),
    ];
    let json = JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).unwrap();
    json.process(json_carrier_batch(&docs))
        .await
        .unwrap()
        .pop()
        .unwrap()
}

#[tokio::test]
async fn select_shapes_and_orders_rows() {
    let proc = SqlProcessor::from_config(&json!({
        "type": "sql",
        "query": "SELECT city, temp_c FROM flow ORDER BY city",
    }))
    .expect("build");

    let out = proc.process(typed_batch().await).await.expect("process");
    assert_eq!(out.len(), 1, "one input batch yields one result batch");

    let rows = batch_to_rows(&out[0]).expect("to rows").rows;
    let cities: Vec<&str> = rows.iter().map(|r| r["city"].as_str().unwrap()).collect();
    assert_eq!(cities, ["berlin", "madrid"], "ORDER BY city applied");
}

#[tokio::test]
async fn empty_batch_is_a_noop() {
    let proc = SqlProcessor::from_config(&json!({
        "type": "sql",
        "query": "SELECT * FROM flow",
    }))
    .expect("build");

    // An Arrow batch with zero rows is a no-op for the SQL processor: it emits
    // nothing rather than planning a query over an empty table.
    let empty = datafusion::arrow::array::RecordBatch::new_empty(std::sync::Arc::new(
        datafusion::arrow::datatypes::Schema::empty(),
    ));
    let out = proc.process(empty).await.expect("process");
    assert!(out.is_empty(), "an empty batch yields no result batch");
}

#[tokio::test]
async fn dml_is_rejected() {
    let proc = SqlProcessor::from_config(&json!({
        "type": "sql",
        "query": "DELETE FROM flow",
    }))
    .expect("build defers planning");

    let result = proc.process(typed_batch().await).await;
    assert!(result.is_err(), "a flow's SQL cannot mutate state");
}

#[tokio::test]
async fn custom_table_name_is_honoured() {
    let proc = SqlProcessor::from_config(&json!({
        "type": "sql",
        "query": "SELECT temp_c FROM readings",
        "table_name": "readings",
    }))
    .expect("build");

    let out = proc.process(typed_batch().await).await.expect("process");
    let rows = batch_to_rows(&out[0]).expect("to rows").rows;
    assert_eq!(rows.len(), 2, "batch registered under the custom table name");
}
