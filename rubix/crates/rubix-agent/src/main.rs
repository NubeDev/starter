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
mod registry;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

    let bundle = rubix_spi::i18n::rubix_bundle()?;
    let catalogue_size: usize = bundle
        .languages()
        .filter_map(|tag| bundle.catalog(tag))
        .map(|cat| cat.messages.len())
        .sum();

    let tools = registry::build_tool_registry();
    let migrations = boot::apply_migrations().await?;

    info!(
        crate_name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        tools = tools.len(),
        skills = rubix_skills::bundled().entries().len(),
        flows = rubix_flows::bundled().entries().len(),
        migrations = migrations.sources_applied,
        migrations_skipped = migrations.skipped,
        i18n_keys = catalogue_size,
        "rubix-agent starting"
    );

    let bind = std::env::var("RUBIX_BIND").unwrap_or_else(|_| "127.0.0.1:8088".into());
    health::serve(&bind).await
}
