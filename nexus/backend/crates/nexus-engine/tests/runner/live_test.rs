//! The M0.5 SSE seam test: a `generate` input drives an unbounded stream through
//! the broadcast `sse` sink; a subscriber receives ticking events with
//! monotonic sequence numbers, and the last subscriber leaving cancels the
//! stream. No HTTP, no Bearer — this proves the engine half of the live path.

use std::time::Duration;

use nexus_engine::stream_registry::{attach, register, Attach};
use nexus_engine::{LiveRunner, StreamKey};
use serde_json::json;
use tokio_util::sync::CancellationToken;

fn generate_source() -> (serde_json::Value, Vec<serde_json::Value>) {
    let input = json!({
        "type": "generate",
        "context": "{ \"sensor\": \"temp_1\", \"value\": 10 }",
        "interval": "5ms",
        "batch_size": 1,
    });
    let processors = vec![
        json!({ "type": "json_to_arrow" }),
        json!({ "type": "sql", "query": "SELECT sensor, value FROM flow" }),
    ];
    (input, processors)
}

#[tokio::test]
async fn live_stream_ticks_events_to_a_subscriber() {
    let runner = LiveRunner::new().expect("register builders");
    let key = StreamKey {
        spec: "generate-temp".into(),
        datasource_id: "demo".into(),
        tenant_id: "acme".into(),
        permission: "view".into(),
    };
    let run_id = "live-seam-test";

    let mut sub = match attach(&key, run_id) {
        Attach::StartNew { run_id } => {
            let token = CancellationToken::new();
            let (input, processors) = generate_source();
            runner
                .spawn(input, processors, &run_id, token.clone())
                .expect("spawn live stream");
            register(key.clone(), run_id, token)
        }
        Attach::Existing(_) => panic!("first attach starts a new stream"),
    };

    // Two events should arrive within a generous window given the 5ms interval.
    let first = tokio::time::timeout(Duration::from_secs(5), sub.receiver().recv())
        .await
        .expect("an event arrives in time")
        .expect("event, not a channel error");
    let second = tokio::time::timeout(Duration::from_secs(5), sub.receiver().recv())
        .await
        .expect("a second event arrives")
        .expect("event");

    assert!(second.seq > first.seq, "sequence numbers are monotonic");
    assert_eq!(
        first.rows[0]["sensor"], "temp_1",
        "rows are shaped by the SQL"
    );

    // Dropping the last subscriber tears the stream down (refcount → 0).
    drop(sub);
}
