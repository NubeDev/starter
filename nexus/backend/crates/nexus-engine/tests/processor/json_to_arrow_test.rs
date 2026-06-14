//! `json_to_arrow` parity and schema-stability: a JSON carrier batch parses into
//! a typed Arrow batch, the column types match the documents, and the schema is
//! fixed from the first batch so later batches cannot drift it.

use nexus_engine::arrow_json::{batch_to_rows, columns_of, json_carrier_batch};
use nexus_engine::core::Processor;
use nexus_engine::processor::JsonToArrow;
use nexus_spi::dto::query::ResultColumnType;
use serde_json::json;

fn processor() -> JsonToArrow {
    JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).expect("build")
}

#[tokio::test]
async fn parses_documents_into_typed_columns() {
    let mut proc = processor();
    let batch = json_carrier_batch(&[json!({ "city": "berlin", "temp_c": 21 }).to_string()]);

    let out = proc.process(batch).await.expect("process");
    assert_eq!(out.len(), 1, "one carrier batch yields one typed batch");

    let columns = columns_of(&out[0]);
    let by_name = |n: &str| columns.iter().find(|c| c.name == n).unwrap().column_type;
    assert_eq!(by_name("city"), ResultColumnType::String);
    assert_eq!(by_name("temp_c"), ResultColumnType::Int);

    let rows = batch_to_rows(&out[0]).expect("to rows").rows;
    assert_eq!(rows[0]["city"], "berlin");
    assert_eq!(rows[0]["temp_c"], 21);
}

#[tokio::test]
async fn empty_carrier_batch_yields_no_output() {
    let mut proc = processor();
    let batch = json_carrier_batch(&[]);
    let out = proc.process(batch).await.expect("process");
    assert!(out.is_empty(), "an empty batch produces nothing downstream");
}

#[tokio::test]
async fn schema_is_fixed_from_the_first_batch() {
    let mut proc = processor();

    // First batch fixes `temp_c` as an integer column.
    let first = json_carrier_batch(&[json!({ "temp_c": 21 }).to_string()]);
    let out = proc.process(first).await.expect("first batch");
    assert_eq!(columns_of(&out[0])[0].column_type, ResultColumnType::Int);

    // A later batch whose `temp_c` is a string cannot coerce to the locked
    // integer schema — it is a processor error, not a silent type change.
    let drifted = json_carrier_batch(&[json!({ "temp_c": "warm" }).to_string()]);
    let result = proc.process(drifted).await;
    assert!(
        result.is_err(),
        "a drifted column type is rejected, not silently re-inferred"
    );
}

#[tokio::test]
async fn declared_schema_pins_columns_up_front() {
    let mut proc = JsonToArrow::from_config(&json!({
        "type": "json_to_arrow",
        "schema": { "fields": [
            { "name": "temp_c", "type": "float" },
            { "name": "device_id", "type": "string" },
        ] },
    }))
    .expect("build with declared schema");

    let batch = json_carrier_batch(&[json!({ "temp_c": 21, "device_id": "d1" }).to_string()]);
    let out = proc.process(batch).await.expect("process");
    let columns = columns_of(&out[0]);
    let by_name = |n: &str| columns.iter().find(|c| c.name == n).unwrap().column_type;
    // The integer literal is coerced to the declared float column.
    assert_eq!(by_name("temp_c"), ResultColumnType::Float);
    assert_eq!(by_name("device_id"), ResultColumnType::String);
}
