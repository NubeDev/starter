//! Shaping a connection-probe failure into the redacted wire outcome.
//!
//! Both the saved-datasource probe (`:id/test`) and the pre-save probe (`/test`)
//! report `{ ok, message, latency_ms }` and must never let a driver error leak
//! the connection secret. The sanitization lives here once so both routes shape a
//! failure identically.

use std::time::Instant;

use nexus_spi::dto::datasource::TestDatasourceResponse;

/// Probe latency in whole milliseconds, saturating rather than wrapping on the
/// (practically impossible) overflow.
pub fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// A failed-probe outcome carrying the sanitized reason. Latency is absent
/// because the probe never completed a round-trip.
pub fn failed(e: &starter_spi::Error) -> TestDatasourceResponse {
    TestDatasourceResponse {
        ok: false,
        message: Some(sanitize(&reason(e))),
        latency_ms: None,
    }
}

/// The user-useful reason for a probe failure. A connect failure surfaces as
/// `Error::Internal { source }`, whose own `Display` is the fixed "internal
/// error" — useless on a Test button. So prefer the underlying source's message
/// (e.g. the driver's "Connection refused"), which is the whole point of the
/// probe, and fall back to the error's own text otherwise.
fn reason(e: &starter_spi::Error) -> String {
    use std::error::Error as _;
    e.source()
        .map(|s| s.to_string())
        .unwrap_or_else(|| e.to_string())
}

/// Keep the headline of a driver/connect error but drop anything past the first
/// line — connection strings and DSNs that some drivers append to multi-line
/// errors never reach the client.
fn sanitize(raw: &str) -> String {
    raw.lines().next().unwrap_or("connection failed").to_string()
}
