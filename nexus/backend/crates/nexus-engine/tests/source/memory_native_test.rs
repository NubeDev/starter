//! Native `memory` source: it replays each configured document as one batch and
//! then signals end-of-stream, so a finite pipeline built on it completes.

use nexus_engine::arrow_json::json_carrier_docs;
use nexus_engine::core::Source;
use nexus_engine::source::MemorySource;
use serde_json::json;

#[tokio::test]
async fn replays_documents_then_ends() {
    let mut source = MemorySource::from_config(&json!({
        "type": "memory",
        "messages": [
            json!({ "n": 1 }).to_string(),
            json!({ "n": 2 }).to_string(),
        ],
    }))
    .expect("build");

    let first = source.read().await.expect("read").expect("a batch");
    assert_eq!(
        json_carrier_docs(&first).unwrap(),
        vec![json!({ "n": 1 }).to_string()]
    );

    let second = source.read().await.expect("read").expect("a batch");
    assert_eq!(
        json_carrier_docs(&second).unwrap(),
        vec![json!({ "n": 2 }).to_string()]
    );

    assert!(
        source.read().await.expect("read").is_none(),
        "an exhausted memory source returns None to end the run"
    );
}

#[tokio::test]
async fn empty_messages_ends_immediately() {
    let mut source = MemorySource::from_config(&json!({ "type": "memory" })).expect("build");
    assert!(source.read().await.expect("read").is_none());
}
