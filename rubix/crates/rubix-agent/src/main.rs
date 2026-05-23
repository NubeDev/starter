//! Rubix backend binary.
//!
//! Boots `starter-observability`, mounts the per-verb REST routers
//! under [`crate::routes`] alongside the `/healthz` probe, and logs
//! a structured startup line announcing the tool / skill / flow
//! registry sizes.
//!
//! This file *wires only*. Any logic that isn't pure glue belongs in
//! `rubix-tools` or upstream in starter. See
//! [docs/design/agent/](../../docs/design/agent/README.md).

use std::sync::Arc;

use anyhow::Result;
use tracing::info;

use rubix_agent::{boot, health, registry, routes};

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = boot::init_tracing()?;

    let bundle = Arc::new(rubix_spi::i18n::rubix_bundle()?);
    let catalogue_size: usize = bundle
        .languages()
        .filter_map(|tag| bundle.catalog(tag))
        .map(|cat| cat.messages.len())
        .sum();

    let tools = registry::build_tool_registry();
    let migrations = boot::apply_migrations().await?;
    let ch_migrations = boot::apply_ch_migrations().await?;
    let mcp = boot::mcp::build_mcp_surface().await?;

    info!(
        crate_name = env!("CARGO_PKG_NAME"),
        version = env!("CARGO_PKG_VERSION"),
        tools = tools.len(),
        mcp_tools = mcp.tools.list().len(),
        skills = rubix_skills::bundled().entries().len(),
        flows = rubix_flows::bundled().entries().len(),
        migrations = migrations.sources_applied,
        migrations_skipped = migrations.skipped,
        ch_migrations_skipped = ch_migrations.skipped,
        i18n_keys = catalogue_size,
        "rubix-agent starting"
    );

    // Keep the MCP router alive across the bind / serve below. The
    // `health::serve` entry point owns the listener; the MCP router
    // is merged into it once the HTTP composition lands. For PR 3
    // we wire the registry + router so the surface is reachable
    // through `starter_mcp::testing::pair` from integration tests
    // and through the binary's `boot::mcp` module in process.
    let _mcp_router = mcp.router;

    let tools_state = routes::tools::ToolsState::new(tools, bundle);
    let app = health::healthz_router().merge(routes::tools::router(tools_state));

    let bind = std::env::var("RUBIX_BIND").unwrap_or_else(|_| "127.0.0.1:8088".into());
    health::serve(&bind, app).await
}
