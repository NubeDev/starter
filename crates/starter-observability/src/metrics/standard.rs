//! The standard metric set every starter-server emits: request
//! counter, latency histogram, in-flight gauge.

use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGauge, Opts, Registry};

/// Counters + histograms maintained by starter-server's middleware.
/// Cloning the struct gives a cheap handle — each metric is internally
/// `Arc`-shared by `prometheus::*Vec`.
#[derive(Clone, Debug)]
pub struct StandardMetrics {
    /// `starter_requests_total{method,path,status}`.
    pub requests_total: IntCounterVec,

    /// `starter_request_duration_seconds{method,path,status}`.
    pub request_duration_seconds: HistogramVec,

    /// `starter_requests_in_flight`.
    pub in_flight: IntGauge,
}

/// Latency-histogram buckets, in seconds. Geometric-ish spread covering
/// the range a typical HTTP handler lives in (1 ms ... 10 s).
const LATENCY_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.010, 0.025, 0.050, 0.100, 0.250, 0.500, 1.0, 2.5, 5.0, 10.0,
];

impl StandardMetrics {
    /// Register the standard set on the given registry. Errors only if
    /// the same metric name is already registered.
    pub fn register(registry: &Registry) -> Result<Self, prometheus::Error> {
        let requests_total = IntCounterVec::new(
            Opts::new(
                "starter_requests_total",
                "Total HTTP requests handled, labelled by method, path, status.",
            ),
            &["method", "path", "status"],
        )?;
        let request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "starter_request_duration_seconds",
                "HTTP request duration in seconds, labelled by method, path, status.",
            )
            .buckets(LATENCY_BUCKETS.to_vec()),
            &["method", "path", "status"],
        )?;
        let in_flight = IntGauge::new(
            "starter_requests_in_flight",
            "Requests currently being processed.",
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(request_duration_seconds.clone()))?;
        registry.register(Box::new(in_flight.clone()))?;

        Ok(Self {
            requests_total,
            request_duration_seconds,
            in_flight,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_clean() {
        let registry = Registry::new();
        let m = StandardMetrics::register(&registry).expect("register");
        m.in_flight.inc();
        m.requests_total
            .with_label_values(&["GET", "/health", "200"])
            .inc();
        m.request_duration_seconds
            .with_label_values(&["GET", "/health", "200"])
            .observe(0.012);
        let families = registry.gather();
        assert_eq!(families.len(), 3);
    }

    #[test]
    fn double_register_is_error() {
        let registry = Registry::new();
        StandardMetrics::register(&registry).unwrap();
        let err = StandardMetrics::register(&registry).unwrap_err();
        let _ = err;
    }
}
