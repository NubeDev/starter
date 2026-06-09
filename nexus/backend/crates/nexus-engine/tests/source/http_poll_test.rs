//! The http_poll input fetches a JSON endpoint and emits it for the pipeline.
//! A local one-shot HTTP server stands in for the upstream API; the flow polls
//! it, shapes the body with SQL, and the row lands in the broadcast sink — the
//! same path a real weather poll would take, minus the external dependency.

use std::time::Duration;

use nexus_engine::sink::broadcast_store;
use nexus_engine::stream_registry::{attach, register, Attach};
use nexus_engine::{LiveRunner, StreamKey};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

/// Serve a fixed JSON body to the next N connections, then stop.
async fn serve_json(body: &'static str, times: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        for _ in 0..times {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let mut buf = [0u8; 1024];
            let _ = sock.read(&mut buf).await;
            let resp = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = sock.write_all(resp.as_bytes()).await;
            let _ = sock.flush().await;
        }
    });
    format!("http://{addr}/")
}

#[tokio::test]
async fn http_poll_feeds_a_stream_from_a_real_endpoint() {
    let url = serve_json(r#"{"city":"berlin","temp_c":21}"#, 4).await;

    // Drive the http_poll input through the live runner into the broadcast sink,
    // shaping the fetched JSON with SQL — exactly a flow's input+pipeline.
    let runner = LiveRunner::new().expect("register builders");
    let key = StreamKey {
        spec: "http-poll-test".into(),
        datasource_id: "weather".into(),
        tenant_id: "acme".into(),
        permission: "view".into(),
    };
    let run_id = "http-poll-run";
    let mut sub = match attach(&key, run_id) {
        Attach::StartNew { run_id } => {
            let token = CancellationToken::new();
            let input = json!({ "type": "http_poll", "url": url, "interval": "1s" });
            let processors = vec![
                json!({ "type": "json_to_arrow" }),
                json!({ "type": "sql", "query": "SELECT city, temp_c FROM flow" }),
            ];
            runner
                .spawn(input, processors, &run_id, token.clone())
                .expect("spawn");
            register(key.clone(), run_id, token)
        }
        Attach::Existing(_) => panic!("first attach starts new"),
    };

    let event = tokio::time::timeout(Duration::from_secs(5), sub.receiver().recv())
        .await
        .expect("event in time")
        .expect("event");
    assert_eq!(event.rows[0]["city"], "berlin");
    assert_eq!(event.rows[0]["temp_c"], 21);

    drop(sub);
    broadcast_store::close(run_id);
}
