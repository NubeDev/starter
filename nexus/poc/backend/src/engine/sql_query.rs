//! Run DataFusion SQL over an inline JSON dataset, through a real ArkFlow stream.
//!
//! Wraps the rows in a `memory` input, parses them with `json_to_arrow`, applies
//! the user's `sql` processor over the `flow` table, and collects the result.

use serde_json::{json, Value};

use super::run::{self, RunOutcome};

/// Build a memory → sql stream for `query` over `rows` and run it.
pub async fn query(sql: &str, rows: &[Value]) -> RunOutcome {
    let messages: Vec<String> = rows.iter().map(|r| r.to_string()).collect();

    let config = json!({
        "input": { "type": "memory", "messages": messages },
        "pipeline": {
            "thread_num": 1,
            "processors": [
                { "type": "json_to_arrow" },
                { "type": "sql", "query": sql }
            ]
        },
        // Replaced with the collector by run_config; present to satisfy the schema.
        "output": { "type": "drop" }
    });

    run::run_config(config, 5_000).await
}
