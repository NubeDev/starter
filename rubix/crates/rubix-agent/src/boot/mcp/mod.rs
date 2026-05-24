//! MCP wiring at boot.
//!
//! Loads every bundled rubix flow via [`rubix_flows::load_all`],
//! registers each on a fresh [`FlowRegistry`] under the
//! `com.rubix.ai-agent` kind (bound here to a
//! [`RubixAiAgentNode`] that wraps starter-flow-node-loop's agent
//! loop), wraps each registered flow as a [`FlowAsTool`] via
//! [`FlowAsTool::from_registry`], and returns the assembled
//! starter-mcp [`ToolRegistry`] + `axum::Router` the binary mounts
//! under `/mcp`.
//!
//! There is no per-flow MCP wiring code here — `FlowAsTool` is the
//! one-line contract that turns each flow into an MCP tool, exactly
//! the property SCOPE R7 calls out. Adding a seventh flow is one
//! `*.yaml` file under `rubix-flows/flows/`; this module needs no
//! edit.
//!
//! Locale propagation is still the load-bearing concern. The
//! `starter-mcp` transports (HTTP and the in-memory test pair) bind
//! the caller's BCP-47 tag on a tokio task-local for the lifetime of
//! one `tools/call`; rubix code reads it via
//! [`starter_mcp::current_locale`]. The flow's seed adapter (see
//! [`register`]) snapshots the locale + the corresponding
//! [`ResolvedPreferences`] onto the input slot so the
//! [`agent_node::RubixAiAgentNode`] reads them without re-parsing
//! `Accept-Language` / `_meta.acceptLanguage`.
//!
//! See [docs/design/flows/](../../../../docs/design/flows/README.md)
//! for the loader contract,
//! [docs/design/i18n-prefs/](../../../../docs/design/i18n-prefs/README.md)
//! for the four-transport translation contract, and
//! [docs/design/agent/](../../../../docs/design/agent/README.md) for
//! the boot order this fits into.

use std::sync::Arc;

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use starter_flow_surfaces::FlowAsTool;

use starter_mcp::registry::ToolRegistry;

mod agent_node;
mod prefs;
mod register;

pub use prefs::prefs_from_locale;

/// The flow id rubix surfaces over MCP for goal-5 background system
/// health checks. Kept as a named constant so the bin/admin paths
/// and integration tests reference a single source of truth.
pub const SCHEDULED_SYSTEM_CHECK_FLOW: &str = "com.rubix.scheduled-system-check";

/// Bundle holding the rubix MCP surface — the
/// [`Arc<ToolRegistry>`](ToolRegistry) the dispatch loop reads tools
/// from and the [`axum::Router`] that mounts the HTTP transport on
/// `POST /mcp`. The binary keeps the registry alive for the lifetime
/// of the process; tests pull only the registry and drive it through
/// the in-memory transport.
pub struct McpSurface {
    /// The starter-mcp tool registry the dispatch loop reads.
    pub tools: Arc<ToolRegistry>,
    /// The axum router exposing `POST /mcp`.
    pub router: axum::Router,
}

/// Build the MCP surface for the rubix agent: load every bundled
/// flow, register them on a fresh [`FlowRegistry`], wrap each as a
/// [`FlowAsTool`] via [`FlowAsTool::from_registry`], hand the
/// resulting tool list to starter-mcp's [`ToolRegistry`], and return
/// the assembled router.
///
/// `ch_client` is threaded into the same per-tool `with_history`
/// wiring the REST composition uses, so MCP-triggered probes persist
/// to the warehouse identically.
pub async fn build_mcp_surface(
    ch_client: Option<Arc<starter_store_clickhouse::ChClient>>,
) -> anyhow::Result<McpSurface> {
    let tools = Arc::new(build_tool_registry(ch_client).await?);
    let router: axum::Router =
        starter_mcp::mcp_router(tools.clone(), starter_mcp::McpHttpOptions::default());
    Ok(McpSurface { tools, router })
}

/// Shared composition step: load every bundled rubix flow and
/// surface them as MCP tools on a fresh [`ToolRegistry`]. Both the
/// HTTP surface ([`build_mcp_surface`]) and the stdio surface (the
/// `rubix-admin mcp` subcommand) call this so the tool catalogue is
/// identical across transports — there is no "stdio-only" tool
/// list.
///
/// Emits one `tracing::info` line summarising how many tools landed
/// so the boot log shows `mcp_tools=N` (expected `N = 6` once the
/// six bundled flows are present).
pub async fn build_tool_registry(
    ch_client: Option<Arc<starter_store_clickhouse::ChClient>>,
) -> anyhow::Result<ToolRegistry> {
    let (registry, flows, engine) = register::build_flow_registry(ch_client).await?;
    let mut tools = ToolRegistry::new();
    for (flow_id, revision) in &flows {
        let tool = FlowAsTool::from_registry(&registry, flow_id, revision, engine.clone())
            .await
            .map_err(|e| anyhow::anyhow!("FlowAsTool::from_registry({flow_id}): {e}"))?;
        tools = tools.register(tool);
    }
    tracing::info!(mcp_tools = flows.len(), "rubix MCP surface assembled");
    Ok(tools)
}

/// Construct the in-memory graph store and bind it on a fresh
/// [`Engine`]. Both [`register::build_flow_registry`] and integration
/// tests share this helper so the engine boot shape is uniform.
pub(crate) fn build_engine() -> Arc<Engine> {
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    Arc::new(Engine::new(graph_store))
}
