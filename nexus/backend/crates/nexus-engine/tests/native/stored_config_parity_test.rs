//! Stored-flow parity: a flow config blob persisted verbatim under the previous
//! engine — the exact `{input, pipeline, output}` JSON with the `type`
//! discriminants the old graph serialised — parses and runs on the native engine
//! with no migration. This is the acceptance proof that flows created before the
//! cutover keep working: the registry kept the same node names on purpose, so a
//! stored config is config, not code to rewrite.
//!
//! The fixture is an opaque string parsed at runtime (not a `json!` literal), so
//! it is the byte-for-byte shape a tenant's saved flow carries in the database.

use nexus_engine::core::{Pipeline, PipelineConfig, RunOutcome};
use nexus_engine::native_registry;
use nexus_engine::sink::store;
use nexus_engine::Caps;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

/// A flow config exactly as it was stored under the old engine: a `memory` input
/// of JSON documents, a `json_to_arrow` then `sql` pipeline, and a `collector`
/// output. The `run_id` placeholder is the one field the runner rewrites per run.
const STORED_FLOW: &str = r#"{
  "input": {
    "type": "memory",
    "messages": [
      "{\"sensor\":\"a\",\"reading\":10}",
      "{\"sensor\":\"b\",\"reading\":20}",
      "{\"sensor\":\"c\",\"reading\":30}"
    ]
  },
  "pipeline": {
    "processors": [
      { "type": "json_to_arrow" },
      { "type": "sql", "query": "SELECT sensor, reading FROM flow ORDER BY sensor" }
    ]
  },
  "output": { "type": "collector", "run_id": "__RUN_ID__" }
}"#;

#[tokio::test]
async fn flow_stored_under_old_engine_runs_unchanged() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let token = CancellationToken::new();
    store::open(&run_id, Caps::unbounded(), token.clone());

    // Substitute only the per-run collector id; everything else is the stored
    // blob untouched, proving the persisted shape needs no migration.
    let raw = STORED_FLOW.replace("__RUN_ID__", &run_id);
    let config: Value = serde_json::from_str(&raw).expect("stored flow is valid JSON");

    let cfg = PipelineConfig::from_value(config).expect("stored config parses on native engine");
    let outcome = Pipeline::build(&native_registry(), &cfg)
        .expect("native registry builds every stored node type")
        .run(token)
        .await
        .expect("stored flow runs to completion");
    assert_eq!(outcome, RunOutcome::Completed, "finite stored flow completes");

    let drained = store::take(&run_id);
    assert_eq!(drained.rows.len(), 3, "all stored rows flow through unchanged");
    let sensors: Vec<&str> = drained
        .rows
        .iter()
        .map(|r| r["sensor"].as_str().unwrap())
        .collect();
    assert_eq!(
        sensors,
        ["a", "b", "c"],
        "rows arrive shaped exactly as the stored SQL specifies"
    );
    let names: Vec<&str> = drained.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["sensor", "reading"],
        "column schema matches the stored projection"
    );
}
