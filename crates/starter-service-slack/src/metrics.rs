//! The prometheus metric set this service registers, per SCOPE R7.
//!
//! Three handles, registered against the [`prometheus::Registry`] the
//! consumer hands in via [`ServiceContext::metrics`](starter_spi::service::ServiceContext):
//!
//! - `starter_service_slack_events_total{kind}` — counter, one bump per
//!   envelope handed to `EventSink::emit`, labelled by the same `kind`
//!   string the sink saw (`slack.message`, `slack.app_mention`, …).
//! - `starter_service_slack_restarts_total{reason}` — counter, one bump
//!   per inner-loop attempt past the first, labelled by what triggered
//!   the restart (`disconnect`, `transport_error`, `slack_api`, …).
//! - `starter_service_slack_running` — gauge, 1 while a websocket is
//!   open and pumping, 0 otherwise.

use prometheus::{IntCounterVec, IntGauge, Opts, Registry};

/// Handles for the metrics [`crate::SlackSocketModeService`] emits.
///
/// Cheap to clone — every prometheus collector is internally `Arc`-shared.
#[derive(Clone)]
pub(crate) struct ServiceMetrics {
    /// `starter_service_slack_events_total{kind}`.
    pub events: IntCounterVec,
    /// `starter_service_slack_restarts_total{reason}`.
    pub restarts: IntCounterVec,
    /// `starter_service_slack_running`.
    pub running: IntGauge,
}

impl ServiceMetrics {
    pub(crate) fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let events = IntCounterVec::new(
            Opts::new(
                "starter_service_slack_events_total",
                "Slack events emitted by starter-service-slack into its EventSink, labelled by kind.",
            ),
            &["kind"],
        )?;
        let restarts = IntCounterVec::new(
            Opts::new(
                "starter_service_slack_restarts_total",
                "Times the starter-service-slack inner connect loop restarted, labelled by reason.",
            ),
            &["reason"],
        )?;
        let running = IntGauge::with_opts(Opts::new(
            "starter_service_slack_running",
            "1 while the starter-service-slack websocket is open and pumping, 0 otherwise.",
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
