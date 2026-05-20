//! The prometheus metric set this tool registers, per SCOPE R7.

use prometheus::{Histogram, HistogramOpts, IntCounterVec, Opts, Registry};

/// Latency-histogram buckets, seconds. GitHub API normally responds
/// in 100–500 ms; buckets stretch from 5 ms to 30 s.
const LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0, 30.0,
];

/// Handles for the metrics [`crate::GitHubCreateIssueTool`] emits.
///
/// Cheap to clone — every prometheus collector is internally `Arc`-shared.
#[derive(Clone)]
pub(crate) struct ToolMetrics {
    /// `starter_tool_github_create_issue_duration_seconds` — call latency.
    pub latency: Histogram,
    /// `starter_tool_github_create_issue_errors_total{kind}` — error counter.
    pub errors: IntCounterVec,
}

impl ToolMetrics {
    pub(crate) fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let latency = Histogram::with_opts(
            HistogramOpts::new(
                "starter_tool_github_create_issue_duration_seconds",
                "Latency of starter-tool-github create-issue calls, in seconds.",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
        )?;
        let errors = IntCounterVec::new(
            Opts::new(
                "starter_tool_github_create_issue_errors_total",
                "Failed starter-tool-github create-issue calls, labelled by failure kind.",
            ),
            &["kind"],
        )?;
        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(errors.clone()))?;
        Ok(Self { latency, errors })
    }
}
