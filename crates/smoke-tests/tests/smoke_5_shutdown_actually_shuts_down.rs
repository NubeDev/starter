//! Smoke test 5 — shutdown actually shuts down.
//!
//! Calling [`ServiceRegistry::shutdown`] must cause every `Service`'s
//! `JoinHandle` to resolve within
//! [`SHUTDOWN_DEADLINE_DEFAULT`](starter_spi::service::SHUTDOWN_DEADLINE_DEFAULT)
//! (5 s, Decision D3). A service that ignores `ServiceContext.shutdown`
//! and has to be force-aborted shows up as
//! [`ServiceShutdownOutcome::Aborted`] in the report — this test fails
//! the build for any such service.
//!
//! We deliberately point every service at an unreachable base URL so
//! they spend the run in their retry/backoff loops. The contract under
//! test is "cooperative exit during backoff", not "happy-path shutdown
//! after a clean event": a backoff that doesn't race the shutdown
//! signal is exactly the bug this smoke test catches.

use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use prometheus::Registry;
use serde_json::Value;
use starter_service_slack::{SlackSocketModeConfig, SlackSocketModeService};
use starter_service_telegram::{TelegramBotConfig, TelegramBotService};
use starter_spi::service::{
    EventSink, ServiceRegistry, ServiceShutdownOutcome, SinkResult, SHUTDOWN_DEADLINE_DEFAULT,
};
use starter_spi::SecretString;

/// Drop-everything sink — events never arrive in this test anyway, the
/// services exit during backoff before they ever connect.
struct NullSink;

#[async_trait]
impl EventSink for NullSink {
    async fn emit(&self, _kind: &str, _payload: Value) -> SinkResult<()> {
        Ok(())
    }
}

/// Base URL guaranteed to refuse connections — port 1 on the loopback
/// is reserved for tcpmux and is closed on every CI image.
const UNREACHABLE_HTTP: &str = "http://127.0.0.1:1";
const UNREACHABLE_WS_HTTP: &str = "http://127.0.0.1:1";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_drains_every_service_within_default_deadline() {
    // Sanity-check the documented constant. If somebody bumps the
    // default to 60s, this test should be visibly the place that
    // notices — Decision D3 makes the constant the SemVer contract.
    assert_eq!(
        SHUTDOWN_DEADLINE_DEFAULT,
        Duration::from_secs(5),
        "SHUTDOWN_DEADLINE_DEFAULT is the smoke-5 contract; changing \
         it is a SemVer-visible edit",
    );

    let metrics = Arc::new(Registry::new());
    let sink: Arc<dyn EventSink> = Arc::new(NullSink);

    let mut services = ServiceRegistry::new()
        .register(SlackSocketModeService::new(SlackSocketModeConfig {
            app_token: SecretString::from("xapp-test".to_string()),
            base_url: UNREACHABLE_WS_HTTP.to_string(),
        }))
        .register(TelegramBotService::new(TelegramBotConfig {
            bot_token: SecretString::from("12345:test".to_string()),
            base_url: UNREACHABLE_HTTP.to_string(),
        }));

    services.start_all(metrics, sink).await.expect("start_all");

    // Let each service enter its retry loop at least once so the test
    // exercises the "shutdown observed during backoff" path, not the
    // "shutdown observed before connect" cheap-exit. 200 ms is enough
    // for one failed connect attempt against a closed loopback port.
    tokio::time::sleep(Duration::from_millis(200)).await;

    let started = Instant::now();
    let report = services.shutdown().await;
    let elapsed = started.elapsed();

    assert!(
        elapsed <= SHUTDOWN_DEADLINE_DEFAULT,
        "shutdown took {elapsed:?}, exceeding SHUTDOWN_DEADLINE_DEFAULT={SHUTDOWN_DEADLINE_DEFAULT:?}",
    );

    // Every service must have observed `ctx.shutdown` and exited
    // cooperatively. A `JoinHandle` left in flight long enough for the
    // registry to abort it is the exact failure smoke 5 forbids.
    let mut aborts: Vec<&str> = Vec::new();
    for (name, outcome) in &report.services {
        if matches!(outcome, ServiceShutdownOutcome::Aborted) {
            aborts.push(name.as_str());
        }
    }
    assert!(
        aborts.is_empty(),
        "the following services did not observe `ctx.shutdown` and \
         were force-aborted at the deadline: {aborts:?}. Smoke test 5 \
         fails any service whose backoff loop does not race the \
         shutdown signal.",
    );

    // Either Clean or Error is fine — Error is expected because we
    // pointed at an unreachable URL. The contract is "the join handle
    // resolved", not "the loop succeeded".
    for (name, outcome) in &report.services {
        match outcome {
            ServiceShutdownOutcome::Clean
            | ServiceShutdownOutcome::Error(_)
            | ServiceShutdownOutcome::JoinError(_) => {}
            ServiceShutdownOutcome::Aborted => unreachable!("checked above; {name}"),
        }
    }
}
