//! The prometheus metric set this tool registers, per SCOPE R7.
//!
//! Both metrics are registered against the [`prometheus::Registry`]
//! the consumer hands to [`crate::GmailSendTool::new`].

use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

/// Latency-histogram buckets, seconds. Gmail's REST surface typically
/// lands in 150–800 ms; the buckets stretch from 5 ms to 30 s so a
/// dashboard can spot tail-latency regressions without re-bucketing.
const LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Handles for the metrics [`crate::GmailSendTool`] emits.
///
/// Cheap to clone — every prometheus collector is internally `Arc`-shared.
#[derive(Clone)]
pub(crate) struct ToolMetrics {
    /// `starter_tool_gmail_send_duration_seconds` — call latency,
    /// including network + Gmail-side processing.
    pub latency: Histogram,
    /// `starter_tool_gmail_send_errors_total{kind}` — error counter,
    /// labelled by failure mode (`transport`, `auth`, `http_status`,
    /// `missing_id`, `message_build`, `bad_input`).
    pub errors: IntCounterVec,
}

impl ToolMetrics {
    pub(crate) fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let latency = Histogram::with_opts(
            HistogramOpts::new(
                "starter_tool_gmail_send_duration_seconds",
                "Latency of starter-tool-gmail users.messages.send calls, in seconds.",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
        )?;
        let errors = IntCounterVec::new(
            Opts::new(
                "starter_tool_gmail_send_errors_total",
                "Failed starter-tool-gmail users.messages.send calls, labelled by failure kind.",
            ),
            &["kind"],
        )?;
        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(errors.clone()))?;
        Ok(Self { latency, errors })
    }
}
