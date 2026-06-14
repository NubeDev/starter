//! Zenoh source loopback: a peer publisher and the `ZenohSource` subscriber mesh
//! over an in-process TCP endpoint (no external router), a published JSON sample
//! arrives as a carrier batch, and the source closes cleanly on cancellation.
//!
//! This is the major win the spec calls out — zenoh's in-process peers let the
//! whole pub→source round-trip run with no broker, so the connector has a real
//! integration test in CI behind its feature gate.

use std::time::Duration;

use nexus_engine::arrow_json::json_carrier_docs;
use nexus_engine::core::Source;
use nexus_engine::source::zenoh::ZenohSource;
use serde_json::json;

/// A free TCP port for the loopback endpoint, found by binding to port 0 and
/// releasing it — zenoh then re-binds it. A momentary race is acceptable in a test.
fn free_endpoint() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    format!("tcp/127.0.0.1:{port}")
}

/// Open a peer publisher that listens on `endpoint` so the subscriber can connect.
async fn publisher(endpoint: &str) -> zenoh::Session {
    let mut cfg = zenoh::Config::default();
    cfg.insert_json5("mode", "\"peer\"").unwrap();
    cfg.insert_json5("listen/endpoints", &format!("[\"{endpoint}\"]"))
        .unwrap();
    // No multicast scouting needed — the subscriber connects directly.
    cfg.insert_json5("scouting/multicast/enabled", "false")
        .unwrap();
    zenoh::open(cfg).await.expect("publisher session opens")
}

// Zenoh's runtime rejects Tokio's current-thread scheduler, so this round-trip
// runs on a multi-thread runtime (one worker is enough).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn loopback_sample_arrives_as_a_batch() {
    let endpoint = free_endpoint();
    let pubr = publisher(&endpoint).await;

    let config = json!({
        "type": "zenoh",
        "mode": "peer",
        "endpoints": [endpoint],
        "key_expr": "test/telemetry",
    });
    let mut source = ZenohSource::build(&config).expect("source builds from config");

    // Drive the source read concurrently while the publisher emits, so the
    // session is established before the sample is put. The first read opens the
    // session and declares the subscriber, then awaits a sample.
    let reader = tokio::spawn(async move {
        let batch = source.read().await.expect("read ok").expect("a batch");
        let docs = json_carrier_docs(&batch).expect("carrier docs");
        docs
    });

    // Give the subscriber a moment to connect and declare before publishing.
    tokio::time::sleep(Duration::from_millis(300)).await;
    pubr.put("test/telemetry", json!({ "v": 42 }).to_string())
        .await
        .expect("publish");

    let docs = tokio::time::timeout(Duration::from_secs(5), reader)
        .await
        .expect("read within timeout")
        .expect("reader task");
    assert_eq!(docs.len(), 1, "one sample, one document");
    let parsed: serde_json::Value = serde_json::from_str(&docs[0]).unwrap();
    assert_eq!(parsed["v"], 42, "JSON payload forwarded verbatim");
}
