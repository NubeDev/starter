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

/// Initialise the global tracing subscriber.
///
/// `filter` is a `RUST_LOG`-style directive string (e.g.
/// `"info,starter_server=debug"`). Pass `"info"` if unsure.
///
/// Returns `Ok(())` if installation succeeded. Installing twice in
/// the same process is an error and returns it — call once at
/// startup.
pub fn init(filter: &str, format: Format) -> Result<(), Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let env_filter = EnvFilter::try_new(filter)?;

    let registry = tracing_subscriber::registry().with(env_filter);

    match format {
        Format::Pretty => registry.with(fmt::layer().compact()).try_init()?,
        Format::Json => registry.with(fmt::layer().json()).try_init()?,
    }

    Ok(())
}
