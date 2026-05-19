//! Subscriber initialisation. Returns a guard the binary holds for
//! its lifetime — when dropped, async writers flush.

/// Output format for tracing subscribers.
#[derive(Debug, Clone, Copy)]
pub enum Format {
    /// Human-friendly compact output. Use for local dev.
    Pretty,
    /// One-line JSON per event. Use in production for log
    /// aggregation pipelines.
    Json,
}

/// Drop-guard returned by [`init`]. Holding it keeps any non-blocking
/// async writers (file appenders, OTLP exporters, …) alive; dropping
/// it triggers the flush.
///
/// Today the subscriber writes directly to stdout, which has no
/// background worker — the guard is a no-op. The type exists so call
/// sites already use the right pattern when a file-appender or OTLP
/// layer is added later (a `_guard` binding at startup will keep the
/// worker alive for the whole process).
#[must_use = "drop the guard at process exit so async writers flush"]
#[derive(Default)]
pub struct TracingGuard {
    _inner: (),
}

/// Initialise the global tracing subscriber.
///
/// `filter` is a `RUST_LOG`-style directive string (e.g.
/// `"info,starter_server=debug"`). Pass `"info"` if unsure.
///
/// Returns a [`TracingGuard`] the caller must keep alive for the
/// lifetime of the process (typically `let _guard = init(...)?` in
/// `main`). Installing twice in the same process is an error.
pub fn init(filter: &str, format: Format) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_new(filter)?;

    let registry = tracing_subscriber::registry().with(env_filter);

    match format {
        Format::Pretty => registry.with(fmt::layer().compact()).try_init()?,
        Format::Json => registry.with(fmt::layer().json()).try_init()?,
    }

    Ok(TracingGuard::default())
}
