//! [`SlackSocketModeService`] — the `Service` impl. Wraps the
//! [`socket_mode`] connect+pump loop in a [`RetryPolicy`] and watches
//! [`ServiceContext.shutdown`](starter_spi::service::ServiceContext)
//! for cooperative exit.

use std::sync::Arc;

use async_trait::async_trait;
use starter_spi::service::{Service, ServiceContext, ServiceHandle};
use starter_spi::{ExposeSecret, Result as SpiResult};
use tokio::sync::watch;

use crate::config::SlackSocketModeConfig;
use crate::error::SlackSocketModeError;
use crate::metrics::ServiceMetrics;
use crate::retry::{RetryPolicy, RetryStep};
use crate::socket_mode::{open_connection, open_connection_url, pump_until_closed, ConnectOutcome};

/// Stable `service.name` label. Keep aligned with dashboard queries.
pub const SERVICE_NAME: &str = "slack-socket-mode";

/// Inbound Slack Service. Opens a socket-mode connection and emits
/// every `events_api` envelope into the consumer's `EventSink` as
/// `slack.<event_type>`.
///
/// Construct once at startup, register into a `ServiceRegistry`. The
/// service holds no state visible outside the spawned task — every
/// runtime knob lives inside [`RetryPolicy`] or comes in via
/// [`ServiceContext`].
pub struct SlackSocketModeService {
    config: SlackSocketModeConfig,
    http: reqwest::Client,
    retry: RetryPolicy,
}

impl SlackSocketModeService {
    /// Build the service with the default [`RetryPolicy`].
    pub fn new(config: SlackSocketModeConfig) -> Self {
        Self::with_client(config, reqwest::Client::new())
    }

    /// Same as [`Self::new`] but accepts a pre-built
    /// [`reqwest::Client`]. Use this when the consumer wants a shared
    /// client across every starter-provider crate.
    pub fn with_client(config: SlackSocketModeConfig, http: reqwest::Client) -> Self {
        Self {
            config,
            http,
            retry: RetryPolicy::default(),
        }
    }

    /// Builder-style override for the retry / circuit-breaker policy.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }
}

#[async_trait]
impl Service for SlackSocketModeService {
    fn name(&self) -> &'static str {
        SERVICE_NAME
    }

    async fn start(&self, ctx: ServiceContext) -> SpiResult<ServiceHandle> {
        // Register metrics on the consumer's prometheus registry up
        // front. Failing here is a programmer error (service constructed
        // twice against the same registry, or another service grabbed
        // our names): surface it as `Internal` rather than letting the
        // spawned task fail silently.
        let metrics =
            ServiceMetrics::register(&ctx.metrics).map_err(|e| starter_spi::Error::Internal {
                source: Box::new(e),
            })?;

        // Clone the bits the spawned task owns so `&self` is not
        // held across the task boundary.
        let endpoint = open_connection_url(&self.config.base_url);
        let app_token = self.config.app_token.expose_secret().to_string();
        let http = self.http.clone();
        let mut retry = self.retry.clone();
        let sink = ctx.sink.clone();
        let mut shutdown = ctx.shutdown;

        let join = tokio::spawn(async move {
            run_loop(
                http,
                endpoint,
                app_token,
                sink,
                &mut shutdown,
                metrics,
                &mut retry,
            )
            .await
        });

        Ok(ServiceHandle::new(join))
    }
}

/// The reconnect loop. Each iteration:
///   1. POST `apps.connections.open` for a fresh `wss_url`.
///   2. Dial the WebSocket and pump frames until it closes or
///      shutdown fires.
///   3. On clean disconnect, reset the retry counter and loop. On
///      failure, ask the retry policy what to do and either sleep or
///      trip the circuit.
async fn run_loop(
    http: reqwest::Client,
    endpoint: String,
    app_token: String,
    sink: Arc<dyn starter_spi::service::EventSink>,
    shutdown: &mut watch::Receiver<bool>,
    metrics: ServiceMetrics,
    retry: &mut RetryPolicy,
) -> SpiResult<()> {
    loop {
        // Cheap try-borrow: if shutdown already fired, exit before we
        // burn a connect attempt.
        if *shutdown.borrow() {
            tracing::info!(
                service.name = SERVICE_NAME,
                "shutdown observed before connect"
            );
            return Ok(());
        }

        let attempt_result =
            connect_and_pump(&http, &endpoint, &app_token, &sink, shutdown, &metrics).await;

        match attempt_result {
            Ok(ConnectOutcome::Shutdown) => {
                tracing::info!(service.name = SERVICE_NAME, "shutdown observed; exiting");
                return Ok(());
            }
            Ok(ConnectOutcome::Disconnected) => {
                // Slack rotates the socket ~every 30 minutes. A clean
                // disconnect after we'd been pumping is normal and
                // must not count toward the circuit.
                retry.record_success();
                metrics.restarts.with_label_values(&["disconnect"]).inc();
                tracing::info!(
                    service.name = SERVICE_NAME,
                    "socket disconnected; reconnecting",
                );
                // No backoff on a clean disconnect — immediately get a
                // fresh wss_url. If apps.connections.open then fails,
                // the failure path below will apply backoff.
                continue;
            }
            Err(err) => {
                let reason = error_reason(&err);
                let err_display = err.to_string();
                match retry.next_step() {
                    RetryStep::Backoff { backoff, attempt } => {
                        metrics.restarts.with_label_values(&[reason]).inc();
                        tracing::warn!(
                            service.name = SERVICE_NAME,
                            error = %err,
                            error.reason = reason,
                            attempt,
                            backoff_ms = backoff.as_millis() as u64,
                            "socket-mode error; will retry",
                        );
                        // Race the backoff against the shutdown signal
                        // so a Ctrl-C does not stall for up to a minute.
                        tokio::select! {
                            biased;
                            res = shutdown.changed() => {
                                if res.is_err() || *shutdown.borrow() {
                                    tracing::info!(
                                        service.name = SERVICE_NAME,
                                        "shutdown observed during backoff",
                                    );
                                    return Ok(());
                                }
                            }
                            _ = tokio::time::sleep(backoff) => {}
                        }
                    }
                    RetryStep::Trip { attempts } => {
                        tracing::error!(
                            service.name = SERVICE_NAME,
                            attempts,
                            last = %err,
                            "socket-mode circuit tripped; exiting",
                        );
                        return Err(SlackSocketModeError::CircuitTripped {
                            attempts,
                            last: err_display,
                        }
                        .into());
                    }
                }
            }
        }
    }
}

async fn connect_and_pump(
    http: &reqwest::Client,
    endpoint: &str,
    app_token: &str,
    sink: &Arc<dyn starter_spi::service::EventSink>,
    shutdown: &mut watch::Receiver<bool>,
    metrics: &ServiceMetrics,
) -> Result<ConnectOutcome, SlackSocketModeError> {
    let wss_url = open_connection(http, endpoint, app_token).await?;
    tracing::info!(service.name = SERVICE_NAME, "socket-mode connection opened",);
    pump_until_closed(&wss_url, shutdown, sink, metrics, SERVICE_NAME).await
}

/// Stable `reason` label for the `starter_service_slack_restarts_total`
/// counter. Keep aligned with the dashboard query.
fn error_reason(err: &SlackSocketModeError) -> &'static str {
    match err {
        SlackSocketModeError::Transport(_) => "transport",
        SlackSocketModeError::HttpStatus { .. } => "http_status",
        SlackSocketModeError::SlackApi(_) => "slack_api",
        SlackSocketModeError::BadWssUrl(_) => "bad_wss_url",
        SlackSocketModeError::WebSocket(_) => "websocket",
        SlackSocketModeError::CircuitTripped { .. } => "circuit_tripped",
    }
}
