//! The standard metric set every starter-server emits: request
//! counter, latency histogram, in-flight gauge.

use prometheus::{HistogramVec, IntCounterVec, IntGauge, Registry};

/// Counters + histograms maintained by starter-server's middleware.
/// One instance per process; cloned references are cheap.
pub struct StandardMetrics {
    /// `requests_total{method,path,status}`.
    pub requests_total: IntCounterVec,

    /// `request_duration_seconds{method,path,status}`.
    pub request_duration_seconds: HistogramVec,

    /// `requests_in_flight`.
    pub in_flight: IntGauge,
}

impl StandardMetrics {
    /// Register the standard set on the given registry.
    /// Errors only if the same metric name is already registered.
    pub fn register(_registry: &Registry) -> Result<Self, prometheus::Error> {
        // TODO(ap): wire actual metric construction once the
        // middleware in `super::super::middleware` lands. The shape
        // is locked in via the doc-comments above.
        todo!("metric construction lands with the middleware in starter-observability v0.2")
    }
}
