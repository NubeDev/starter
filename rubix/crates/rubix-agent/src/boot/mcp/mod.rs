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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use starter_ext_host::ExtensionRegistry;
use starter_ext_spi::ExtensionId;
use starter_ext_supervisor::SupervisorHandle;
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow_spi::graph::GraphStore;
use starter_flow_surfaces::FlowAsTool;

use starter_mcp::registry::ToolRegistry;

/// Per-request timeout the MCP adapter passes to each
/// `ProcessExtensionToolBinding`. Generous enough that a process
/// extension can do its own I/O without tripping, tight enough that a
/// hung extension does not stall `tools/call`. SCOPE OQ-4: the
/// rubix-agent end of the MCP wiring picks the value; extensions
/// cannot override it.
const EXTENSION_TOOL_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Input handed to [`build_mcp_surface`] when the agent has an
/// extension host wired in. Lets the MCP transport adapter
/// (`starter-ext-mcp`) walk the validated registry and emit one MCP
/// tool per `contributes.tools[]` entry, dispatching to the matching
/// supervisor handle for process-flavour extensions.
///
/// `None` (the laptop / no-DB path) means the MCP surface still works
/// — it just won't include extension-contributed tools, because no
/// extensions were loaded.
pub struct ExtensionMcpContext {
    /// The sealed registry, shared with `ExtensionAdmin`.
    pub registry: Arc<ExtensionRegistry>,
    /// Live supervisor handles for autostarted process-flavour
    /// extensions, keyed by [`ExtensionId`] as
    /// [`starter_ext_mcp::register_process_tools`] expects.
    pub process_handles: HashMap<ExtensionId, Arc<SupervisorHandle>>,
}

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
    pg_pool: Option<starter_store_postgres::pool::Pool>,
    ext: Option<&ExtensionMcpContext>,
    runtime: Option<&crate::boot::FlowRuntime>,
) -> anyhow::Result<McpSurface> {
    let tools = Arc::new(build_tool_registry(ch_client, pg_pool, ext, runtime).await?);
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
    pg_pool: Option<starter_store_postgres::pool::Pool>,
    ext: Option<&ExtensionMcpContext>,
    runtime: Option<&crate::boot::FlowRuntime>,
) -> anyhow::Result<ToolRegistry> {
    let ext_registry = ext.map(|c| c.registry.as_ref());
    let (registry, flows, engine) =
        register::build_flow_registry(ch_client, pg_pool, runtime, ext_registry).await?;
    let mut tools = ToolRegistry::new();
    for (flow_id, revision) in &flows {
        let tool = FlowAsTool::from_registry(&registry, flow_id, revision, engine.clone())
            .await
            .map_err(|e| anyhow::anyhow!("FlowAsTool::from_registry({flow_id}): {e}"))?;
        tools = tools.register(tool);
    }

    // SCOPE OQ-4: walk every validated process-flavour extension and
    // register its `contributes.tools[]` into the same `ToolRegistry`
    // the bundled `FlowAsTool` entries land in. The adapter is the
    // single transport seam; without this call extensions appear under
    // `GET /extensions` but `tools/list` is silently incomplete.
    let mut ext_registered: usize = 0;
    if let Some(ctx) = ext {
        let (next, outcome, result) = starter_ext_mcp::register_process_tools(
            &ctx.registry,
            &ctx.process_handles,
            EXTENSION_TOOL_REQUEST_TIMEOUT,
            tools,
        );
        tools = next;
        ext_registered = outcome.tools_registered;
        if let Err(e) = result {
            // Per-tool failures are aggregated by the adapter — log
            // and continue. The successful tools are already in
            // `tools`; an operator can read per-id failure detail via
            // `GET /api/v1/extensions/<id>/events`.
            tracing::warn!(
                target: "rubix.boot.extensions.mcp",
                error = %e,
                "one or more extension tools failed to wire into MCP",
            );
        }
        tracing::info!(
            target: "rubix.boot.extensions.mcp",
            extensions_seen = outcome.extensions_seen,
            tools_registered = outcome.tools_registered,
            tools_skipped_non_process = outcome.tools_skipped_non_builtin,
            "extension tools wired into MCP",
        );
    }

    tracing::info!(
        mcp_tools = flows.len() + ext_registered,
        flow_tools = flows.len(),
        extension_tools = ext_registered,
        "rubix MCP surface assembled",
    );
    Ok(tools)
}

/// Construct the in-memory graph store and bind it on a fresh
/// [`Engine`]. Both [`register::build_flow_registry`] and integration
/// tests share this helper so the engine boot shape is uniform.
///
/// `runtime`, when provided, attaches its persistent
/// `NodeStateStore` and `FlowEventSink` (the same
/// `FlowSubscriptionRegistry` the SSE route subscribes to) so
/// every surface-driven run (`FlowAsTool` / `FlowAsService`)
/// shares state and event fan-out with the always-on flow runtime.
pub(crate) fn build_engine(
    runtime: Option<&crate::boot::FlowRuntime>,
) -> Arc<Engine> {
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let mut engine = Engine::new(graph_store);
    if let Some(rt) = runtime {
        engine = engine
            .with_node_state_store(rt.state_store.clone())
            .with_event_sink(rt.subscriptions.clone() as Arc<dyn starter_flow::FlowEventSink>);
    }
    Arc::new(engine)
}
