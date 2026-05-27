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
///
/// ## Why this is *not* a `tracing-appender::non_blocking` writer
///
/// An earlier iteration of this module routed stdout through
/// `tracing_appender::non_blocking` to move log formatting off the
/// worker threads. That introduced a strictly worse failure mode:
/// if the background writer thread ever panics (we observed the
/// known `tracing-subscriber` "tried to clone a span that already
/// closed" assertion under heavy concurrent span use — see
/// <https://github.com/tokio-rs/tracing/issues/1656>), the bounded
/// channel stops draining, every subsequent `info!()` call blocks
/// waiting for channel space, and the entire tokio runtime wedges
/// with every worker parked on a futex. Sync stdout is slower
/// under load but it cannot deadlock the runtime.
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
/// When the `RUST_LOG` environment variable is set, it wins over the
/// `filter` argument so operators can crank verbosity without
/// recompiling. The argument is the default; the env var is the
/// override.
///
/// Returns a [`TracingGuard`] the caller must keep alive for the
/// lifetime of the process (typically `let _guard = init(...)?` in
/// `main`). Installing twice in the same process is an error.
pub fn init(filter: &str, format: Format) -> Result<TracingGuard, Box<dyn std::error::Error>> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter, Layer};

    // CRITICAL: when the tokio-console feature is on, the EnvFilter
    // must be a **per-layer filter on the fmt layer only**, not a
    // registry-wide global filter. console_subscriber's own layer
    // consumes the `tokio=trace` and `runtime=trace` targets that
    // tokio's instrumentation emits — if a global EnvFilter at
    // `info` level is wired (as `.with(env_filter)` on the registry
    // would do), tokio's TRACE events are filtered out **before**
    // the console layer ever sees them, and tokio-console clients
    // receive empty heartbeats forever. We hit this in production:
    // RUST_LOG=info silently disabled the entire console surface.
    //
    // Per-layer filters: `fmt_layer.with_filter(env_filter)` only
    // affects the fmt layer. console_subscriber's `spawn()` returns
    // a layer with its own internal filter that subscribes to
    // tokio's targets at TRACE.
    let env_filter = match std::env::var("RUST_LOG") {
        Ok(env) if !env.trim().is_empty() => EnvFilter::try_new(&env)?,
        _ => EnvFilter::try_new(filter)?,
    };

    let registry = tracing_subscriber::registry();
    #[cfg(feature = "tokio-console")]
    let registry = registry.with(console_subscriber::spawn());

    match format {
        Format::Pretty => registry
            .with(fmt::layer().compact().with_filter(env_filter))
            .try_init()?,
        Format::Json => registry
            .with(fmt::layer().json().with_filter(env_filter))
            .try_init()?,
    }

    #[cfg(feature = "tokio-console")]
    tracing::info!(
        target: "starter.observability.tokio_console",
        bind = "127.0.0.1:6669 (default; override via TOKIO_CONSOLE_BIND)",
        "tokio-console layer active — connect with `tokio-console`",
    );

    Ok(TracingGuard::default())
}
