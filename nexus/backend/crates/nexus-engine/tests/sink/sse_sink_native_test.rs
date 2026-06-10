//! Native `sse` sink: each written batch is published as the next event with a
//! monotonic sequence number, so a `Last-Event-ID` resume sees no gap — the same
//! seq behaviour as the ArkFlow sse sink.

use nexus_engine::arrow_json::json_carrier_batch;
use nexus_engine::core::Sink;
use nexus_engine::processor::JsonToArrow;
use nexus_engine::core::Processor;
use nexus_engine::sink::broadcast_store;
use nexus_engine::sink::SseSink;
use serde_json::json;

#[tokio::test]
async fn publishes_batches_with_monotonic_sequence_numbers() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let channel = broadcast_store::open(&run_id);
    let mut subscriber = channel.subscribe();

    let mut sink = SseSink::from_config(&json!({ "type": "sse", "run_id": run_id }))
        .expect("resolve reserved channel");

    // Shape two single-row batches and publish each.
    let json = JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).unwrap();
    for value in [json!({ "sensor": "a", "value": 1 }), json!({ "sensor": "b", "value": 2 })] {
        let typed = json
            .process(json_carrier_batch(&[value.to_string()]))
            .await
            .unwrap()
            .pop()
            .unwrap();
        sink.write(&typed).await.expect("write");
    }

    let first = subscriber.recv().await.expect("first event");
    let second = subscriber.recv().await.expect("second event");
    assert_eq!(first.seq, 0, "sequence starts at zero");
    assert_eq!(second.seq, 1, "sequence is monotonic across batches");
    assert_eq!(first.rows[0]["sensor"], "a");
    assert_eq!(second.rows[0]["sensor"], "b");

    broadcast_store::close(&run_id);
}
