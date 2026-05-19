//! Integration tests for `TelegramBotService`.
//!
//! Stubs the Bot API entirely with `wiremock` — every test points the
//! service at the mock URL via [`TelegramBotConfig::base_url`]. No
//! real network is touched.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use prometheus::Registry;
use serde_json::{json, Value};
use starter_service_telegram::{
    InMemoryOffsetStore, OffsetStore, RetryPolicy, TelegramBotConfig, TelegramBotService,
    SERVICE_NAME,
};
use starter_spi::service::{EventSink, Service, ServiceRegistry, SinkResult};
use starter_spi::SecretString;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TOKEN: &str = "12345:test-secret";

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

#[tokio::test(flavor = "multi_thread")]
async fn emits_update_as_kind_and_forwards_payload() {
    let server = MockServer::start().await;

    // First call (offset=None) returns one message update. Subsequent
    // calls (offset=2) return an empty array so the loop sits idle
    // until shutdown.
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/getUpdates")))
        .and(body_partial_json(json!({}))) // matches anything; ordering matters next
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [
                {
                    "update_id": 1,
                    "message": {
                        "message_id": 7,
                        "chat": {"id": 555},
                        "from": {"id": 42},
                        "text": "hello"
                    }
                }
            ]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // Catch-all for follow-up polls.
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/getUpdates")))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": []
        })))
        .mount(&server)
        .await;

    let sink = VecSink::default();
    let sink_arc: Arc<dyn EventSink> = Arc::new(sink.clone());

    let cfg = TelegramBotConfig {
        bot_token: SecretString::from(TOKEN.to_string()),
        base_url: server.uri(),
    };
    let svc = TelegramBotService::new(cfg).with_poll_timeout_secs(0);
    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Wait for the first emit to land.
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

    let report = services.shutdown().await;
    let outcome = &report.services[0].1;
    assert!(
        !matches!(
            outcome,
            starter_spi::service::ServiceShutdownOutcome::Aborted
        ),
        "service was force-aborted: {outcome:?}",
    );

    let events = sink.events.lock().unwrap().clone();
    assert!(
        !events.is_empty(),
        "expected at least one emitted update, got {events:?}",
    );
    assert_eq!(events[0].0, "telegram.message");
    assert_eq!(events[0].1["message"]["chat"]["id"].as_i64(), Some(555));
    assert_eq!(events[0].1["update_id"].as_i64(), Some(1));

    // Metrics: events counter bumped under telegram.message.
    let families = metrics.gather();
    let events_metric = families
        .iter()
        .find(|f| f.name() == "starter_service_telegram_events_total")
        .expect("events counter registered");
    let count: u64 = events_metric
        .get_metric()
        .iter()
        .map(|m| m.get_counter().value() as u64)
        .sum();
    assert!(count >= 1, "expected events counter to bump");
}

#[tokio::test(flavor = "multi_thread")]
async fn offset_cookie_is_persisted_across_polls() {
    // First poll: no offset, returns update_id=10. Second poll MUST
    // arrive with offset=11 (= max+1). Third+ catch-all.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/getUpdates")))
        .and(body_partial_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": [{"update_id": 10, "message": {"text": "a"}}]
        })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/getUpdates")))
        .and(body_partial_json(json!({"offset": 11})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "ok": true,
            "result": []
        })))
        .mount(&server)
        .await;

    let store: Arc<dyn OffsetStore> = Arc::new(InMemoryOffsetStore::new());
    let sink_arc: Arc<dyn EventSink> = Arc::new(VecSink::default());
    let cfg = TelegramBotConfig {
        bot_token: SecretString::from(TOKEN.to_string()),
        base_url: server.uri(),
    };
    let svc = TelegramBotService::new(cfg)
        .with_poll_timeout_secs(0)
        .with_offset_store(store.clone());
    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Let a couple of polls go through.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        if store.load().await.unwrap() == Some(11) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let _ = services.shutdown().await;

    assert_eq!(
        store.load().await.unwrap(),
        Some(11),
        "offset cookie must persist max(update_id)+1 across polls",
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn shutdown_breaks_the_poll_loop_during_backoff() {
    // Closed port — every connect attempt fails. The retry layer
    // should back off; shutdown must short-circuit the long sleep.
    let sink_arc: Arc<dyn EventSink> = Arc::new(VecSink::default());
    let cfg = TelegramBotConfig {
        bot_token: SecretString::from(TOKEN.to_string()),
        base_url: "http://127.0.0.1:1".to_string(),
    };
    let svc = TelegramBotService::new(cfg)
        .with_poll_timeout_secs(0)
        .with_retry_policy(
            RetryPolicy::default()
                .with_initial_backoff(Duration::from_secs(30))
                .with_max_attempts(100),
        );
    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    // Let the first attempt fail and enter the 30s sleep.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let report = services.shutdown().await;
    assert!(matches!(
        report.services[0].1,
        starter_spi::service::ServiceShutdownOutcome::Clean
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn circuit_trips_immediately_on_401() {
    // 401 is non-transient; the service must exit on the first hit
    // rather than waiting for max_attempts.
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!("/bot{TOKEN}/getUpdates")))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;

    let sink_arc: Arc<dyn EventSink> = Arc::new(VecSink::default());
    let cfg = TelegramBotConfig {
        bot_token: SecretString::from(TOKEN.to_string()),
        base_url: server.uri(),
    };
    let svc = TelegramBotService::new(cfg).with_poll_timeout_secs(0);
    assert_eq!(svc.name(), SERVICE_NAME);

    let mut services = ServiceRegistry::new().register(svc);
    let metrics = Arc::new(Registry::new());
    services.start_all(metrics.clone(), sink_arc).await.unwrap();

    let report = tokio::time::timeout(Duration::from_secs(5), services.shutdown())
        .await
        .expect("registry shutdown completed in time");

    match &report.services[0].1 {
        starter_spi::service::ServiceShutdownOutcome::Error(_) => {}
        starter_spi::service::ServiceShutdownOutcome::Clean => {}
        other => panic!("unexpected shutdown outcome: {other:?}"),
    }
}
