//! Rubix backend binary.
//!
//! Wiring composition site. Boots `starter-observability`, loads the
//! layered config, applies migrations, builds the tool registry, and
//! composes one axum Router from the sub-routers.
//!
//! This file delegates to:
//!   - [`hooks`] — panic hook, runtime canary, USR1 metrics
//!   - [`services`] — config, migrations, pools, extensions, tools, MCP
//!   - [`compose`] — router assembly + `health::serve`

use anyhow::Result;

use rubix_agent::boot;

mod compose;
mod hooks;
mod services;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

    hooks::install_panic_hook();
    let diag = hooks::install_runtime_diagnostics()?;

    let svc = services::boot_services().await?;
    compose::compose_and_serve(svc, diag.canary).await
}
