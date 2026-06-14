//! End-to-end parity for the native pipeline: a `memory → json_to_arrow → sql →
//! collector` run over the native registry returns the same rows, columns, and
//! truncation behaviour the query seam does (see
//! `tests/runner/query_test.rs`), proving the RW-02 ports compose.

use nexus_engine::core::{Pipeline, PipelineConfig, RunOutcome};
use nexus_engine::native_registry;
use nexus_engine::sink::store;
use nexus_engine::Caps;
use serde_json::{json, Value};
use tokio_util::sync::CancellationToken;

/// Build the pipeline JSON for a `memory` source carrying `rows`, shaped by
/// `query`, into the collector reserved under `run_id`.
fn config(rows: &[Value], query: &str, run_id: &str) -> Value {
    let messages: Vec<String> = rows.iter().map(|r| r.to_string()).collect();
    json!({
        "input": { "type": "memory", "messages": messages },
        "pipeline": { "processors": [
            { "type": "json_to_arrow" },
            { "type": "sql", "query": query },
        ] },
        "output": { "type": "collector", "run_id": run_id },
    })
}

#[tokio::test]
async fn finite_run_returns_real_rows_and_completes() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    store::open(&run_id, Caps::unbounded(), token.clone());

    let rows = vec![
        json!({ "city": "berlin", "temp_c": 21 }),
        json!({ "city": "madrid", "temp_c": 33 }),
    ];
    let cfg = PipelineConfig::from_value(config(
        &rows,
        "SELECT city, temp_c FROM flow ORDER BY city",
        &run_id,
    ))
    .expect("parse config");

    let outcome = Pipeline::build(&native_registry(), &cfg)
        .expect("build")
        .run(token)
        .await
        .expect("run");
    assert_eq!(outcome, RunOutcome::Completed, "finite source completes");

    let drained = store::take(&run_id);
    assert_eq!(drained.rows.len(), 2, "both rows returned");
    assert!(!drained.truncated, "an uncapped run is not truncated");
    let cities: Vec<&str> = drained
        .rows
        .iter()
        .map(|r| r["city"].as_str().unwrap())
        .collect();
    assert_eq!(
        cities,
        ["berlin", "madrid"],
        "rows arrive shaped by the SQL"
    );
    let names: Vec<&str> = drained.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["city", "temp_c"],
        "column schema derives from Arrow"
    );
}

#[tokio::test]
async fn row_cap_truncates_instead_of_buffering_unbounded() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    store::open(&run_id, Caps::rows(10), token.clone());

    let rows: Vec<Value> = (0..1000).map(|i| json!({ "n": i })).collect();
    let cfg = PipelineConfig::from_value(config(&rows, "SELECT n FROM flow", &run_id))
        .expect("parse config");

    // The run ends via the cap firing the token; that is a clean Cancelled
    // outcome, not an error.
    Pipeline::build(&native_registry(), &cfg)
        .expect("build")
        .run(token)
        .await
        .expect("run");

    let drained = store::take(&run_id);
    assert!(
        drained.truncated,
        "hitting the cap is reported as truncated"
    );
    assert!(
        (drained.rows.len() as u64) <= 10,
        "the collector stops at the cap, it does not buffer all 1000 rows"
    );
}
