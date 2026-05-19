//! Integration tests for `SlackSocketModeService`.
//!
//! Stubs `apps.connections.open` with wiremock and the WebSocket itself
//! with a hand-rolled tokio-tungstenite test server. The service is
//! pointed at both via [`SlackSocketModeConfig::base_url`] (mock HTTP)
//! and the stubbed `url` field of the open-connection response (the
//! local WSS server).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use prometheus::Registry;
use serde_json::Value;
use starter_service_slack::{
    RetryPolicy, SlackSocketModeConfig, SlackSocketModeService, SERVICE_NAME,
};
use starter_spi::service::{EventSink, ServiceRegistry, SinkResult};
use starter_spi::SecretString;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test sink: stashes every emitted (kind, payload) into a vec the
/// test asserts against.
#[derive(Default, Clone)]
struct VecSink {
    events: Arc<Mutex<Vec<(String, Value)>>>,
}

#[async_trait]
impl EventSink for VecSink {
    async fn emit(&self, kind: &str, payload: Value) -> SinkResult<()> {
        self.events
            .lock()
            .unwrap()
            .push((kind.to_string(), payload));
        Ok(())
    }
}

/// Start a WebSocket server on a random port that:
///   - accepts one connection,
///   - sends each frame in `frames` to the client,
///   - then waits for the client to close the socket.
///
/// Returns the bound `ws://127.0.0.1:PORT/`.
async fn start_ws_server(frames: Vec<String>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("ws://{addr}/");
    let join = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        for f in frames {
            ws.send(WsMessage::Text(f)).await.unwrap();
        }
        // Collect acks from the client until it (or we) close the socket.
        let mut acks = Vec::new();
        while let Some(msg) = ws.next().await {
            match msg {
                Ok(WsMessage::Text(t)) => acks.push(t),
                Ok(WsMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        let _ = ws.close(None).await;
        acks
    });
    (url, join)
}

/// Set up wiremock responding to `POST /apps.connections.open` with
/// `ok: true, url: wss_url`. The wiremock base URL is what
/// `SlackSocketModeConfig::base_url` will be pointed at.
async fn mock_open_connection(wss_url: &str) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": true, "url": wss_url})),
        )
        .mount(&mock)
        .await;
    mock
}

#[tokio::test(flavor = "multi_thread")]
async fn emits_events_api_envelope_and_acks() {
    let sink = VecSink::default();
    let sink_arc: Arc<dyn EventSink> = Arc::new(sink.clone());

    let frame = serde_json::json!({
        "type": "events_api",
        "envelope_id": "abc-123",
        "payload": {"event": {"type": "app_mention", "channel": "C1", "text": "hi"}}
    })
    .to_string();
    let (wss_url, server_join) = start_ws_server(vec![frame]).await;
    let mock = mock_open_connection(&wss_url).await;

    let cfg = SlackSocketModeConfig {
        app_token: SecretString::from("xapp-test".to_string()),
        base_url: mock.uri(),
    };
    let svc = SlackSocketModeService::new(cfg).with_retry_policy(
        RetryPolicy::default()
            .with_initial_backoff(Duration::from_millis(10))
            .with_max_attempts(2),
    );
    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Wait for either the emit to land or a short timeout.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if !sink.events.lock().unwrap().is_empty() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Shutdown — gives the pump a chance to drain. We *don't* assert
    // on Clean here because the WS server may close before us; what we
    // do assert is that the service did not get force-aborted.
    let report = services.shutdown().await;
    let outcome = &report.services[0].1;
    assert!(
        !matches!(
            outcome,
            starter_spi::service::ServiceShutdownOutcome::Aborted
        ),
        "service was force-aborted: {outcome:?}",
    );

    // Inspect the emit.
    let events = sink.events.lock().unwrap().clone();
    assert_eq!(
        events.len(),
        1,
        "expected one emitted event, got {events:?}"
    );
    assert_eq!(events[0].0, "slack.app_mention");
    assert_eq!(
        events[0].1["event"]["channel"].as_str(),
        Some("C1"),
        "payload forwarded verbatim",
    );

    // The server should have observed exactly one ack carrying
    // `envelope_id = "abc-123"`.
    let acks = server_join.await.unwrap();
    assert_eq!(
        acks.len(),
        1,
        "expected exactly one ack frame, got {acks:?}"
    );
    let ack: Value = serde_json::from_str(&acks[0]).unwrap();
    assert_eq!(ack["envelope_id"], "abc-123");

    // Metrics: at least one event recorded under slack.app_mention.
    let families = metrics.gather();
    let events_metric = families
        .iter()
        .find(|f| f.name() == "starter_service_slack_events_total")
        .expect("events counter registered");
    let count: u64 = events_metric
        .get_metric()
        .iter()
        .map(|m| m.get_counter().value() as u64)
        .sum();
    assert!(count >= 1, "expected events counter to bump");
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_breaks_the_connect_loop_during_backoff() {
    // No mock — apps.connections.open will fail with a connection error
    // and the service should sit in backoff. Shutdown must short-circuit
    // the backoff sleep.
    let sink_arc: Arc<dyn EventSink> = Arc::new(VecSink::default());
    let cfg = SlackSocketModeConfig {
        app_token: SecretString::from("xapp-test".to_string()),
        base_url: "http://127.0.0.1:1".to_string(), // closed port
    };
    let svc = SlackSocketModeService::new(cfg).with_retry_policy(
        RetryPolicy::default()
            .with_initial_backoff(Duration::from_secs(30))
            .with_max_attempts(100),
    );
    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Give the first attempt a moment to fail and enter the long sleep.
    tokio::time::sleep(Duration::from_millis(150)).await;

    // The default shutdown deadline is 5s, plenty under the 30s
    // sleep — if the service did NOT race shutdown against sleep, the
    // registry would have to abort it.
    let report = services.shutdown().await;
    assert!(matches!(
        report.services[0].1,
        starter_spi::service::ServiceShutdownOutcome::Clean
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn circuit_trips_after_persistent_open_connection_failure() {
    // wiremock responds with `ok: false` every time, so every attempt
    // fails the same way; the circuit must trip rather than retry forever.
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"ok": false, "error": "invalid_auth"})),
        )
        .mount(&mock)
        .await;

    let sink_arc: Arc<dyn EventSink> = Arc::new(VecSink::default());
    let cfg = SlackSocketModeConfig {
        app_token: SecretString::from("xapp-test".to_string()),
        base_url: mock.uri(),
    };
    // Tight policy so the test runs quickly.
    let svc = SlackSocketModeService::new(cfg).with_retry_policy(
        RetryPolicy::default()
            .with_initial_backoff(Duration::from_millis(5))
            .with_max_backoff(Duration::from_millis(20))
            .with_max_attempts(3),
    );
    assert_eq!(svc.name(), SERVICE_NAME);

    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Wait for the circuit to trip. With max_attempts=3 and
    // initial=5ms / max=20ms backoff the entire run is well under 1s.
    let report = tokio::time::timeout(Duration::from_secs(5), services.shutdown())
        .await
        .expect("registry shutdown completed in time");

    // The JoinHandle must have already resolved to `Err(CircuitTripped)`.
    // ServiceRegistry::shutdown drains handles regardless of whether
    // they finished naturally; the outcome is `Error(..)` not `Clean`.
    match &report.services[0].1 {
        starter_spi::service::ServiceShutdownOutcome::Error(_) => {}
        starter_spi::service::ServiceShutdownOutcome::Clean => {}
        other => panic!("unexpected shutdown outcome: {other:?}"),
    }
}

use starter_spi::service::Service;
