//! The M0 engine-seam test: prove that a finite ArkFlow `Stream` driven through
//! the bounded collector returns real rows, terminates cleanly, and honours the
//! row cap by reporting truncation rather than buffering without limit.
//!
//! No database is needed to prove the seam — a `memory` input carrying known
//! JSON rows exercises the exact path a `sql` datasource will: input → arrow →
//! SQL pipeline → collector → JSON.

use nexus_engine::{Caps, QueryRunner};
use serde_json::json;

/// Build the `memory`-input + `json_to_arrow` + `sql` processor chain the seam
/// test runs. Mirrors how the query route shapes a non-SQL input source.
fn memory_input(rows: &[serde_json::Value]) -> (serde_json::Value, Vec<serde_json::Value>) {
    let messages: Vec<String> = rows.iter().map(|r| r.to_string()).collect();
    let input = json!({ "type": "memory", "messages": messages });
    let processors = vec![json!({ "type": "json_to_arrow" })];
    (input, processors)
}

#[tokio::test]
async fn finite_stream_returns_real_rows_and_terminates() {
    let runner = QueryRunner::new().expect("register engine builders");
    let rows = vec![
        json!({ "city": "berlin", "temp_c": 21 }),
        json!({ "city": "madrid", "temp_c": 33 }),
    ];
    let (input, mut processors) = memory_input(&rows);
    processors
        .push(json!({ "type": "sql", "query": "SELECT city, temp_c FROM flow ORDER BY city" }));

    let outcome = runner
        .run(input, processors, Caps::unbounded())
        .await
        .expect("query runs to completion");

    assert_eq!(outcome.stats.row_count, 2, "both rows returned");
    assert!(!outcome.stats.truncated, "an uncapped run is not truncated");
    let cities: Vec<&str> = outcome
        .rows
        .iter()
        .map(|r| r["city"].as_str().unwrap())
        .collect();
    assert_eq!(
        cities,
        ["berlin", "madrid"],
        "rows arrive shaped by the SQL"
    );
    let names: Vec<&str> = outcome.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(
        names,
        ["city", "temp_c"],
        "column schema derives from Arrow"
    );
}

#[tokio::test]
async fn row_cap_truncates_instead_of_buffering_unbounded() {
    let runner = QueryRunner::new().expect("register engine builders");
    let rows: Vec<serde_json::Value> = (0..1000).map(|i| json!({ "n": i })).collect();
    let (input, mut processors) = memory_input(&rows);
    processors.push(json!({ "type": "sql", "query": "SELECT n FROM flow" }));

    let outcome = runner
        .run(input, processors, Caps::rows(10))
        .await
        .expect("capped query still returns an outcome");

    assert!(
        outcome.stats.truncated,
        "hitting the cap is reported as truncated"
    );
    assert!(
        outcome.stats.row_count <= 10,
        "the collector stops at the cap, it does not buffer all 1000 rows"
    );
}
