//! Factory for the single shared `prometheus::Registry`.

use prometheus::Registry;

/// Create a fresh prometheus registry.
///
/// One per process. Consumers pass this into `starter-server` so the
/// `/metrics` route exports both starter-owned and consumer-owned
/// metrics from the same scrape.
pub fn registry() -> Registry {
    Registry::new()
}
