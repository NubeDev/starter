//! Process startup helpers — tracing initialisation.

use anyhow::Result;
use starter_observability::tracing::{self as obs_tracing, Format};

/// Initialise structured logging. RUST_LOG controls the filter
/// (defaults to `info`). Pretty format for now; JSON arrives once
/// the binary runs under a real supervisor.
///
/// Returns the guard `init` hands back; `main()` keeps it alive for
/// the process lifetime. We use `impl Sized` to avoid naming the
/// type — it isn't re-exported from the `tracing` module today.
pub fn init_tracing() -> Result<impl Sized> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());
    obs_tracing::init(&filter, Format::Pretty)
        .map_err(|e| anyhow::anyhow!("init tracing: {e}"))
}
