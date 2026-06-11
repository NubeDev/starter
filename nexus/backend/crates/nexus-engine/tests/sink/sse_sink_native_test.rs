//! Native `sse` sink: each written batch is published as the next event with a
//! monotonic sequence number, so a `Last-Event-ID` resume sees no gap.

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
    let mut json = JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).unwrap();
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

/// RW-08 load-shed guard: a subscriber that falls more than the broadcast buffer
/// behind is dropped for ITS OWN receiver only (a `Lagged` error it recovers from
/// by skipping ahead) — the producer never blocks and other subscribers are
/// unaffected. The monotonic seq numbers make the skipped range observable, which
/// is what the `Last-Event-ID` resume contract reports as the gap.
#[tokio::test]
async fn slow_subscriber_lags_alone_and_the_seq_gap_is_visible() {
    let run_id = uuid::Uuid::new_v4().to_string();
    let channel = broadcast_store::open(&run_id);
    let mut slow = channel.subscribe();

    let mut sink = SseSink::from_config(&json!({ "type": "sse", "run_id": run_id }))
        .expect("resolve reserved channel");

    // Publish well past the broadcast buffer depth (256) without the slow
    // subscriber draining, so its receiver overruns. The producer must not block.
    let mut json = JsonToArrow::from_config(&json!({ "type": "json_to_arrow" })).unwrap();
    for i in 0..1000 {
        let typed = json
            .process(json_carrier_batch(&[json!({ "n": i }).to_string()]))
            .await
            .unwrap()
            .pop()
            .unwrap();
        sink.write(&typed).await.expect("publish never blocks on a slow subscriber");
    }

    // The slow subscriber's first recv reports it lagged — the load-shed for that
    // receiver alone — then it resumes from a later event. The seq it next reads
    // is far past zero: the gap the resume contract surfaces.
    let resumed_seq = loop {
        match slow.recv().await {
            Ok(event) => break event.seq,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                assert!(skipped > 0, "lag reports how many events were shed");
                continue;
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                panic!("channel should still be open with buffered events");
            }
        }
    };
    assert!(resumed_seq > 0, "the slow subscriber skipped a visible seq gap");

    // A subscriber that attaches now sees only fresh events, proving the producer
    // stayed healthy through the other subscriber's lag.
    let mut fresh = channel.subscribe();
    let typed = json
        .process(json_carrier_batch(&[json!({ "n": 1000 }).to_string()]))
        .await
        .unwrap()
        .pop()
        .unwrap();
    sink.write(&typed).await.expect("write after lag");
    let event = fresh.recv().await.expect("fresh subscriber receives current event");
    assert_eq!(event.rows[0]["n"], 1000, "fresh subscriber is unaffected by the lag");

    broadcast_store::close(&run_id);
}
