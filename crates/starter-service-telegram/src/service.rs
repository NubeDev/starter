//! [`TelegramBotService`] — the `Service` impl. Wraps the
//! [`long_poll`](crate::long_poll) loop in a [`RetryPolicy`], honours
//! [`ServiceContext.shutdown`](starter_spi::service::ServiceContext)
//! for cooperative exit, and persists the `getUpdates` cookie via
//! an [`OffsetStore`].

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use starter_spi::service::{EventSink, Service, ServiceContext, ServiceHandle};
use starter_spi::{ExposeSecret, Result as SpiResult};
use tokio::sync::watch;

use crate::config::{TelegramBotConfig, LONG_POLL_TIMEOUT_SECS};
use crate::error::TelegramBotError;
use crate::long_poll::{get_updates_url, poll_once, PolledUpdate};
use crate::metrics::ServiceMetrics;
use crate::offset::{InMemoryOffsetStore, OffsetStore};
use crate::retry::{RetryPolicy, RetryStep};

/// Stable `service.name` label. Keep aligned with dashboard queries.
pub const SERVICE_NAME: &str = "telegram-long-poll";

/// HTTP client timeout. Must exceed `LONG_POLL_TIMEOUT_SECS` by a
/// margin large enough to cover the last-chunk flush from Telegram;
/// 45s matches the codeless reference.
const HTTP_TIMEOUT: Duration = Duration::from_secs(45);

/// Inbound Telegram Service. Drives the `getUpdates` long-poll and
/// emits each update into the consumer's `EventSink` as
/// `telegram.<update_type>`.
///
/// Construct once at startup, register into a `ServiceRegistry`. The
/// service holds no state visible outside the spawned task — every
/// runtime knob lives inside [`RetryPolicy`], [`OffsetStore`], or
/// comes in via [`ServiceContext`].
pub struct TelegramBotService {
    config: TelegramBotConfig,
    http: reqwest::Client,
    retry: RetryPolicy,
    offset_store: Arc<dyn OffsetStore>,
    poll_timeout_secs: u64,
}

impl TelegramBotService {
    /// Build the service with the default [`RetryPolicy`] and the
    /// default in-memory [`OffsetStore`].
    pub fn new(config: TelegramBotConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self::with_client(config, http)
    }

    /// Same as [`Self::new`] but accepts a pre-built
    /// [`reqwest::Client`]. The caller is responsible for setting a
    /// timeout that exceeds the long-poll wait — see
    /// [`HTTP_TIMEOUT`].
    pub fn with_client(config: TelegramBotConfig, http: reqwest::Client) -> Self {
        Self {
            config,
            http,
            retry: RetryPolicy::default(),
            offset_store: Arc::new(InMemoryOffsetStore::new()),
            poll_timeout_secs: LONG_POLL_TIMEOUT_SECS,
        }
    }

    /// Builder-style override for the retry / circuit-breaker policy.
    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Builder-style override for the offset store. v0.1 only ships
    /// the in-memory backend; this seam exists so an at-rest backend
    /// can land without a breaking API change (see crate-level docs).
    pub fn with_offset_store(mut self, store: Arc<dyn OffsetStore>) -> Self {
        self.offset_store = store;
        self
    }

    /// Builder-style override for the long-poll timeout. Useful for
    /// tests; production callers should leave the default.
    pub fn with_poll_timeout_secs(mut self, secs: u64) -> Self {
        self.poll_timeout_secs = secs;
        self
    }
}

#[async_trait]
impl Service for TelegramBotService {
    fn name(&self) -> &'static str {
        SERVICE_NAME
    }

    async fn start(&self, ctx: ServiceContext) -> SpiResult<ServiceHandle> {
        // Register metrics on the consumer's prometheus registry up
        // front. A failure here is a programmer error (two services
        // registered with the same names against the same registry).
        let metrics =
            ServiceMetrics::register(&ctx.metrics).map_err(|e| starter_spi::Error::Internal {
                source: Box::new(e),
            })?;

        // Clone the bits the spawned task owns so `&self` is not
        // held across the task boundary.
        let url = get_updates_url(&self.config.base_url, self.config.bot_token.expose_secret());
        let http = self.http.clone();
        let mut retry = self.retry.clone();
        let store = self.offset_store.clone();
        let timeout_secs = self.poll_timeout_secs;
        let sink = ctx.sink.clone();
        let mut shutdown = ctx.shutdown;

        let join = tokio::spawn(async move {
            run_loop(
                http,
                url,
                store,
                timeout_secs,
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

/// The long-poll reconnect loop. Each iteration:
///   1. Load the current offset from the store.
///   2. POST `getUpdates(offset, timeout=poll_timeout_secs)`, racing
///      the HTTP future against `ctx.shutdown`.
///   3. On success: emit each update into the sink, advance the
///      offset, reset the retry counter.
///   4. On failure: ask the retry policy what to do, then either
///      backoff (racing the sleep against shutdown) or trip.
#[allow(clippy::too_many_arguments)]
async fn run_loop(
    http: reqwest::Client,
    url: String,
    store: Arc<dyn OffsetStore>,
    timeout_secs: u64,
    sink: Arc<dyn EventSink>,
    shutdown: &mut watch::Receiver<bool>,
    metrics: ServiceMetrics,
    retry: &mut RetryPolicy,
) -> SpiResult<()> {
    loop {
        if *shutdown.borrow() {
            tracing::info!(service.name = SERVICE_NAME, "shutdown observed before poll",);
            return Ok(());
        }

        // Best-effort offset load. A backend failure here is logged
        // and we fall back to None — better to risk a re-delivery
        // than to wedge the loop on a transient store error.
        let offset = match store.load().await {
            Ok(o) => o,
            Err(err) => {
                tracing::warn!(
                    service.name = SERVICE_NAME,
                    error = %err,
                    "telegram: offset load failed; polling without offset",
                );
                None
            }
        };

        metrics.running.set(1);
        let poll_result = tokio::select! {
            biased;
            res = shutdown.changed() => {
                metrics.running.set(0);
                if res.is_err() || *shutdown.borrow() {
                    tracing::info!(
                        service.name = SERVICE_NAME,
                        "shutdown observed during poll",
                    );
                    return Ok(());
                }
                // Sender flipped without setting true — keep polling.
                continue;
            }
            r = poll_once(&http, &url, offset, timeout_secs) => r,
        };
        metrics.running.set(0);

        match poll_result {
            Ok(updates) => {
                retry.record_success();
                if !updates.is_empty() {
                    let next_offset = updates.iter().map(|u| u.update_id).max().map(|m| m + 1);
                    emit_updates(&updates, &sink, &metrics).await;
                    if let Some(next) = next_offset {
                        if let Err(err) = store.store(next).await {
                            tracing::warn!(
                                service.name = SERVICE_NAME,
                                error = %err,
                                next_offset = next,
                                "telegram: offset store failed; \
                                 continuing with in-task value",
                            );
                        }
                    }
                }
            }
            Err(err) => {
                let reason = error_reason(&err);
                let err_display = err.to_string();
                // Non-transient errors (401 / 404) trip the circuit
                // immediately rather than waiting for `max_attempts`.
                if err.is_fatal() {
                    let attempts = retry.trip_immediately();
                    metrics.restarts.with_label_values(&[reason]).inc();
                    tracing::error!(
                        service.name = SERVICE_NAME,
                        attempts,
                        last = %err,
                        "telegram: non-transient error; tripping circuit",
                    );
                    return Err(TelegramBotError::CircuitTripped {
                        attempts,
                        last: err_display,
                    }
                    .into());
                }
                match retry.next_step() {
                    RetryStep::Backoff { backoff, attempt } => {
                        metrics.restarts.with_label_values(&[reason]).inc();
                        tracing::warn!(
                            service.name = SERVICE_NAME,
                            error = %err,
                            error.reason = reason,
                            attempt,
                            backoff_ms = backoff.as_millis() as u64,
                            "telegram: long-poll error; will retry",
                        );
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
                            "telegram: retry circuit tripped; exiting",
                        );
                        return Err(TelegramBotError::CircuitTripped {
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

/// Emit every polled update into the sink. Per Decision D4 we
/// log-and-continue on individual sink errors — back-pressure is the
/// sink-fan-out helper's concern at the consumer side; the long-poll
/// must not stall on a slow downstream.
async fn emit_updates(
    updates: &[PolledUpdate],
    sink: &Arc<dyn EventSink>,
    metrics: &ServiceMetrics,
) {
    for u in updates {
        match sink.emit(&u.kind, u.payload.clone()).await {
            Ok(()) => {
                metrics.events.with_label_values(&[&u.kind]).inc();
            }
            Err(err) => {
                tracing::warn!(
                    service.name = SERVICE_NAME,
                    error = %err,
                    kind = %u.kind,
                    update_id = u.update_id,
                    "telegram: event sink emit failed",
                );
            }
        }
    }
}

/// Stable `reason` label for the
/// `starter_service_telegram_restarts_total` counter. Keep aligned
/// with the dashboard query.
fn error_reason(err: &TelegramBotError) -> &'static str {
    match err {
        TelegramBotError::Transport(_) => "transport",
        TelegramBotError::HttpStatus { .. } => "http_status",
        TelegramBotError::BotApi(_) => "bot_api",
        TelegramBotError::CircuitTripped { .. } => "circuit_tripped",
    }
}
