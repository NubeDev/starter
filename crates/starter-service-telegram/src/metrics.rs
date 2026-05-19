//! The prometheus metric set this service registers, per SCOPE R7.
//!
//! Three handles, registered against the [`prometheus::Registry`] the
//! consumer hands in via
//! [`ServiceContext::metrics`](starter_spi::service::ServiceContext):
//!
//! - `starter_service_telegram_events_total{kind}` — counter, one bump
//!   per update handed to `EventSink::emit`, labelled by the same
//!   `kind` string the sink saw (`telegram.message`,
//!   `telegram.edited_message`, …).
//! - `starter_service_telegram_restarts_total{reason}` — counter, one
//!   bump per inner-loop retry, labelled by what triggered the retry
//!   (`transport`, `http_status`, `bot_api`).
//! - `starter_service_telegram_running` — gauge, 1 while the loop is
//!   actively polling, 0 while sleeping in backoff or stopped.

use prometheus::{IntCounterVec, IntGauge, Opts, Registry};

/// Handles for the metrics [`crate::TelegramBotService`] emits.
///
/// Cheap to clone — every prometheus collector is internally `Arc`-shared.
#[derive(Clone)]
pub(crate) struct ServiceMetrics {
    /// `starter_service_telegram_events_total{kind}`.
    pub events: IntCounterVec,
    /// `starter_service_telegram_restarts_total{reason}`.
    pub restarts: IntCounterVec,
    /// `starter_service_telegram_running`.
    pub running: IntGauge,
}

impl ServiceMetrics {
    pub(crate) fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let events = IntCounterVec::new(
            Opts::new(
                "starter_service_telegram_events_total",
                "Telegram updates emitted by starter-service-telegram into its EventSink, labelled by kind.",
            ),
            &["kind"],
        )?;
        let restarts = IntCounterVec::new(
            Opts::new(
                "starter_service_telegram_restarts_total",
                "Times the starter-service-telegram long-poll loop retried, labelled by reason.",
            ),
            &["reason"],
        )?;
        let running = IntGauge::with_opts(Opts::new(
            "starter_service_telegram_running",
            "1 while the starter-service-telegram long-poll loop is actively polling, 0 otherwise.",
        ))?;
        registry.register(Box::new(events.clone()))?;
        registry.register(Box::new(restarts.clone()))?;
        registry.register(Box::new(running.clone()))?;
        Ok(Self {
            events,
            restarts,
            running,
        })
    }
}
