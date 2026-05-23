//! Rubix backend binary.
//!
//! Boots `starter-observability`, serves `/healthz` via
//! `starter-server`, and logs a structured startup line announcing
//! the tool / skill / flow registry sizes.
//!
//! This file *wires only*. Any logic that isn't pure glue belongs in
//! `rubix-tools` or upstream in starter. See
//! [docs/design/agent/](../../docs/design/agent/README.md).

use anyhow::Result;
use tracing::info;

mod boot;
mod health;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

    info!(
        crate_name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        tools = 0,
        skills = rubix_skills::bundled().entries().len(),
        flows = rubix_flows::bundled().entries().len(),
        "rubix-agent starting"
    );

    let bind = std::env::var("RUBIX_BIND").unwrap_or_else(|_| "127.0.0.1:8088".into());
    health::serve(&bind).await
}
