//! The prometheus metric set this tool registers, per SCOPE R7.
//!
//! Both metrics are registered against the [`prometheus::Registry`]
//! the consumer hands to [`crate::TelegramSendMessageTool::new`].

use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

/// Latency-histogram buckets, seconds. The Bot API typically lands in
/// 100–400 ms; the buckets stretch from 5 ms to 30 s so a dashboard
/// can spot tail-latency regressions without re-bucketing.
const LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Handles for the metrics [`crate::TelegramSendMessageTool`] emits.
///
/// Cheap to clone — every prometheus collector is internally `Arc`-shared.
#[derive(Clone)]
pub(crate) struct ToolMetrics {
    /// `starter_tool_telegram_send_message_duration_seconds` — call
    /// latency, including network + Telegram-side processing.
    pub latency: Histogram,
    /// `starter_tool_telegram_send_message_errors_total{kind}` — error
    /// counter, labelled by failure mode (`transport`, `rate_limited`,
    /// `http_status`, `bot_api`, `missing_result`, `bad_input`).
    pub errors: IntCounterVec,
}

impl ToolMetrics {
    pub(crate) fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let latency = Histogram::with_opts(
            HistogramOpts::new(
                "starter_tool_telegram_send_message_duration_seconds",
                "Latency of starter-tool-telegram sendMessage calls, in seconds.",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
        )?;
        let errors = IntCounterVec::new(
            Opts::new(
                "starter_tool_telegram_send_message_errors_total",
                "Failed starter-tool-telegram sendMessage calls, labelled by failure kind.",
            ),
            &["kind"],
        )?;
        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(errors.clone()))?;
        Ok(Self { latency, errors })
    }
}
