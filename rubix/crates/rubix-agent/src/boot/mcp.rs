//! MCP wiring at boot.
//!
//! Loads every bundled rubix flow via [`rubix_flows::load_all`],
//! registers each on a fresh [`FlowRegistry`] under the
//! `com.rubix.ai-agent` kind (a stub [`NodeBehavior`] until Block C
//! binds `starter-flow-node-loop`'s real `AiAgentNode`), wraps each
//! registered flow as a [`FlowAsTool`] via
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
//! [`starter_mcp::current_locale`]. The flow's seed adapter snapshots
//! the locale + the corresponding [`ResolvedPreferences`] onto the
//! input slot so the eventual `AiAgentNode` (Block C) reads them
//! without re-parsing `Accept-Language` / `_meta.acceptLanguage`.
//!
//! See [docs/design/flows/](../../../docs/design/flows/README.md) for
//! the loader contract, [docs/design/i18n-prefs/](../../../docs/design/i18n-prefs/README.md)
//! for the four-transport translation contract, and
//! [docs/design/agent/](../../../docs/design/agent/README.md) for the
//! boot order this fits into.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use rubix_flows::AI_AGENT_KIND_ID;

use starter_flow::definition::body::FlowBody;
use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotRef, SlotValue,
};
use starter_flow_surfaces::{FlowAsTool, FlowRegistration, FlowRegistry};

use starter_mcp::registry::ToolRegistry;
use starter_spi::i18n::LanguageTag;
use starter_spi::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::Unit;

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
pub async fn build_mcp_surface() -> anyhow::Result<McpSurface> {
    let tools = Arc::new(build_tool_registry().await?);
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
pub async fn build_tool_registry() -> anyhow::Result<ToolRegistry> {
    let (registry, flows, engine) = build_flow_registry().await?;
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

/// Lower-level entry point exposed so integration tests can drive
/// the same wiring without standing up the HTTP listener. Returns
/// the registry, the list of `(flow_id, revision)` pairs that were
/// landed, and an [`Arc<Engine>`](Engine) bound to a fresh
/// in-memory graph store.
pub async fn build_flow_registry(
) -> anyhow::Result<(Arc<FlowRegistry>, Vec<(FlowId, FlowRevisionId)>, Arc<Engine>)> {
    // -- 1. Engine on a fresh in-memory graph store. The terminal-
    //       slot read-back in `FlowAsTool` reads through this store.
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph_store));

    // -- 2. NodeKindRegistry carrying the ai-agent stub. Block C
    //       replaces this stub with starter-flow-node-loop's real
    //       AiAgentNode wired to a Claude runner; until then any
    //       attempt to invoke a flow surfaces a clear "not wired"
    //       NodeError (registration still succeeds because the
    //       resolver only needs the kind to be *registered*).
    let kinds = NodeKindRegistry::new();
    let ai_agent_kind = KindId::new(AI_AGENT_KIND_ID)
        .map_err(|e| anyhow::anyhow!("invalid {AI_AGENT_KIND_ID} kind id: {e}"))?;
    let stub: Arc<dyn NodeBehavior> = Arc::new(AiAgentStubNode {
        kind: ai_agent_kind.clone(),
    });
    kinds
        .register(stub)
        .await
        .map_err(|e| anyhow::anyhow!("register {AI_AGENT_KIND_ID} stub: {e}"))?;

    // -- 3. FlowRegistry seeded from every bundled YAML.
    let registry = Arc::new(FlowRegistry::new());
    let mut flows = Vec::new();
    let triples = rubix_flows::load_all()
        .map_err(|e| anyhow::anyhow!("rubix_flows::load_all: {e}"))?;

    for (flow_id, revision, body) in triples {
        register_one(&registry, &kinds, &flow_id, revision, body).await?;
        flows.push((flow_id, revision));
    }

    Ok((registry, flows, engine))
}

/// Register one `(flow_id, revision, body)` triple with the shared
/// adapter shape every bundled flow uses: a single seed slot
/// (`payload`) carrying a JSON object snapshotting the caller's
/// locale + resolved preferences, and a single terminal slot
/// (`out`) read back by the output adapter.
async fn register_one(
    registry: &FlowRegistry,
    kinds: &NodeKindRegistry,
    flow_id: &FlowId,
    revision: FlowRevisionId,
    body: FlowBody,
) -> anyhow::Result<()> {
    let root = body
        .nodes
        .first()
        .ok_or_else(|| anyhow::anyhow!("flow `{flow_id}` has zero nodes after conversion"))?;
    let seed_slot = SlotRef::new(root.id.clone(), rubix_flows::DEFAULT_SEED_SLOT);
    let output_slot = SlotRef::new(root.id.clone(), rubix_flows::DEFAULT_OUTPUT_SLOT);

    let tool_id = KindId::new(flow_id.to_string())
        .map_err(|e| anyhow::anyhow!("flow `{flow_id}` is not a valid KindId: {e}"))?;

    let seed_slot_for_adapter = seed_slot.clone();
    let seed: starter_flow_surfaces::SeedAdapter = Arc::new(move |input: &Value| {
        // The locale task-local is bound by starter-mcp's dispatch
        // wrapper before this closure runs; reading it here is the
        // U1 contract (no Accept-Language parsing in rubix, no
        // manual `LanguageTag` threading). Falls back to "en" if
        // the dispatcher did not bind a locale (e.g. an MCP client
        // that did not supply `_meta.acceptLanguage`).
        let lang = starter_mcp::current_locale()
            .unwrap_or_else(|| LanguageTag::parse("en").expect("'en' parses"));
        let prefs = prefs_from_locale(&lang);
        let payload = json!({
            "lang": prefs.language,
            "locale": lang.as_str(),
            "prefs": prefs,
            "input": input.clone(),
        });
        vec![(seed_slot_for_adapter.clone(), SlotValue::Json(payload))]
    });

    let output_key = format!("{}.{}", output_slot.node, output_slot.slot);
    let output: starter_flow_surfaces::OutputAdapter = Arc::new(move |out: &SlotMap| -> Value {
        match out.get(&output_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => Value::Null,
        }
    });

    let description = format!("{flow_id} — bundled rubix flow rooted at an ai-agent node.");
    let spec = FlowRegistration::new(body, revision, tool_id, flow_id.to_string(), description)
        .terminal_slots(vec![output_slot])
        .input_schema(json!({"type": "object"}))
        .output_schema(json!({"type": "object"}))
        .with_adapters(seed, output);

    registry
        .register(spec, kinds)
        .await
        .map_err(|e| anyhow::anyhow!("register `{flow_id}`: {e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// ai-agent stub kind (Block A).
// ---------------------------------------------------------------------------

/// Stub [`NodeBehavior`] bound under [`AI_AGENT_KIND_ID`] for the
/// duration of Block A. Resolution-time wiring needs the kind to be
/// *registered* (so `FlowRegistry::register` succeeds and every
/// bundled flow lands as an MCP tool); invocation is what Block C
/// fixes by replacing this stub with `starter-flow-node-loop`'s
/// real `AiAgentNode` wrapped around a Claude runner.
struct AiAgentStubNode {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for AiAgentStubNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, _input: SlotMap) -> Result<SlotMap, NodeError> {
        Err(NodeError::Backend(format!(
            "{AI_AGENT_KIND_ID}: not wired yet — Block A registers the kind so flows surface \
             as MCP tools; Block C binds the real AiAgentNode behaviour"
        )))
    }
}

// ---------------------------------------------------------------------------
// Locale → ResolvedPreferences mapping.
// ---------------------------------------------------------------------------

/// Map a BCP-47 [`LanguageTag`] to a [`ResolvedPreferences`] whose
/// timezone, date format, time format, and language reflect a
/// reasonable default for the tag's region subtag.
///
/// Called from the seed adapter at MCP `tools/call` dispatch time
/// and from sibling code in [`crate::routes::tools`] /
/// [`crate::bin::rubix_admin`] that wants the same locale → prefs
/// mapping outside the flow path.
pub fn prefs_from_locale(tag: &LanguageTag) -> ResolvedPreferences {
    let raw = tag.as_str();
    let (timezone, locale, language, date_format, time_format) = match raw {
        "en-US" => (
            "America/New_York",
            "en-US",
            "en",
            DateFormat::MdySlash,
            TimeFormat::H24,
        ),
        "es-AR" => (
            "America/Argentina/Buenos_Aires",
            "es-AR",
            "es",
            DateFormat::DmySlash,
            TimeFormat::H24,
        ),
        _ => {
            // Fall back to the language-only subtag for the i18n
            // catalogue lookup; UTC / ISO date stay neutral so the
            // operator at least sees a parseable timestamp.
            let lang = raw.split('-').next().unwrap_or("en");
            (
                "UTC",
                raw,
                if lang.is_empty() { "en" } else { lang },
                DateFormat::IsoYMD,
                TimeFormat::H24,
            )
        }
    };
    ResolvedPreferences {
        timezone: timezone.to_owned(),
        locale: locale.to_owned(),
        language: language.to_owned(),
        unit_system: UnitSystem::Metric,
        temperature_unit: Unit::Celsius,
        pressure_unit: Unit::Kilopascal,
        speed_unit: Unit::MeterPerSecond,
        length_unit: Unit::Meter,
        mass_unit: Unit::Kilogram,
        date_format,
        time_format,
        week_start: WeekStart::Monday,
        number_format: NumberFormat::SpaceComma,
        currency: "USD".to_owned(),
        theme: Theme::System,
    }
}
