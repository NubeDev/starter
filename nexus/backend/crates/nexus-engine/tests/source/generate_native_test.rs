//! Native `generate` source: a `count`-bounded config emits exactly that many
//! documents and then ends, so a finite flow test terminates; the first read is
//! immediate.

use nexus_engine::arrow_json::json_carrier_docs;
use nexus_engine::core::Source;
use nexus_engine::source::GenerateSource;
use serde_json::json;

#[tokio::test]
async fn count_bounds_the_stream() {
    let mut source = GenerateSource::from_config(&json!({
        "type": "generate",
        "context": json!({ "v": 1 }).to_string(),
        "interval": "5ms",
        "batch_size": 1,
        "count": 2,
    }))
    .expect("build");

    assert!(source.read().await.expect("read").is_some(), "first emit");
    assert!(source.read().await.expect("read").is_some(), "second emit");
    assert!(
        source.read().await.expect("read").is_none(),
        "the count bound ends the source"
    );
}

#[tokio::test]
async fn batch_size_emits_multiple_documents_per_read() {
    let mut source = GenerateSource::from_config(&json!({
        "type": "generate",
        "context": json!({ "v": 1 }).to_string(),
        "interval": "5ms",
        "batch_size": 3,
        "count": 3,
    }))
    .expect("build");

    let batch = source.read().await.expect("read").expect("a batch");
    assert_eq!(
        json_carrier_docs(&batch).unwrap().len(),
        3,
        "batch_size documents per read"
    );
    assert!(
        source.read().await.expect("read").is_none(),
        "one batch of three reaches the count of three"
    );
}
