//! MCP wiring at boot.
//!
//! Builds the rubix [`FlowRegistry`] containing
//! `com.rubix.scheduled-system-check`, plugs it into starter-mcp's
//! [`ToolRegistry`] via the one-line
//! [`FlowAsTool::from_registry`] contract, and returns the assembled
//! tool registry + `axum::Router` the binary mounts under `/mcp`.
//!
//! Locale propagation is the load-bearing concern here. The
//! `starter-mcp` transports (HTTP and the in-memory test pair) bind
//! the caller's BCP-47 tag on a tokio task-local for the lifetime of
//! one `tools/call`; rubix code reads it via
//! [`starter_mcp::current_locale`]. We never thread a `LanguageTag`
//! through call sites by hand, and we never re-parse
//! `Accept-Language` / `_meta.acceptLanguage` here — that is U1's
//! job upstream.
//!
//! See [docs/design/i18n-prefs/](../../../docs/design/i18n-prefs/README.md)
//! for the four-transport translation contract and
//! [docs/design/agent/](../../../docs/design/agent/README.md) for the
//! boot order this fits into.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotRef, SlotValue,
};
use starter_flow_surfaces::{
    FlowAsTool, FlowRegistration, FlowRegistry,
};

use starter_flow::definition::body::{FlowBody, NodeDecl};

use starter_mcp::registry::ToolRegistry;
use starter_spi::i18n::{Diagnostic, DiagnosticParam, LanguageTag, MessageKey};
use starter_spi::preferences::{
    DateFormat, NumberFormat, ResolvedPreferences, Theme, TimeFormat, UnitSystem, WeekStart,
};
use starter_spi::units::Unit;

/// The flow id rubix surfaces over MCP for goal-5 background system
/// health checks.
pub const SCHEDULED_SYSTEM_CHECK_FLOW: &str = "com.rubix.scheduled-system-check";

/// Reverse-DNS id of the bundled diagnostic-renderer node kind. Lives
/// outside the reserved `starter.flow.*` prefix so the public
/// [`NodeKindRegistry::register`] entry point accepts it.
const DIAG_RENDER_KIND: &str = "com.rubix.diag-render";

/// Node id used inside the bundled flow body. Must be reverse-DNS.
const RENDER_NODE_ID: &str = "com.rubix.render";

/// Slot the seed adapter writes; also the only trigger slot on the
/// render node.
const SEED_SLOT: &str = "payload";

/// Slot the render node writes the rendered diagnostic to.
const OUTPUT_SLOT: &str = "out";

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

/// Build the MCP surface for the rubix agent: register every bundled
/// flow on a fresh [`FlowRegistry`], wrap each as a
/// [`FlowAsTool`] via [`FlowAsTool::from_registry`], hand the
/// resulting tool list to starter-mcp's [`ToolRegistry`], and return
/// the assembled router.
///
/// Stage-1 of PR 3 only mounts `com.rubix.scheduled-system-check`;
/// stages 4+ register the remaining five goal flows the same way.
pub async fn build_mcp_surface() -> anyhow::Result<McpSurface> {
    let (registry, flow_id, revision, engine) = build_flow_registry().await?;
    let tool = FlowAsTool::from_registry(&registry, &flow_id, &revision, engine)
        .await
        .map_err(|e| anyhow::anyhow!("FlowAsTool::from_registry: {e}"))?;

    let tools = Arc::new(ToolRegistry::new().register(tool));
    let router: axum::Router =
        starter_mcp::mcp_router(tools.clone(), starter_mcp::McpHttpOptions::default());
    Ok(McpSurface { tools, router })
}

/// Lower-level entry point exposed so integration tests can drive
/// the same wiring without standing up the HTTP listener. Returns
/// the registry + the `(flow_id, revision)` pair the tests pass to
/// [`FlowAsTool::from_registry`] alongside an
/// [`Arc<Engine>`](Engine).
pub async fn build_flow_registry(
) -> anyhow::Result<(Arc<FlowRegistry>, FlowId, FlowRevisionId, Arc<Engine>)> {
    // -- 1. Engine on a fresh in-memory graph store. The terminal-
    //       slot read-back in `FlowAsTool` reads through this store.
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Arc::new(Engine::new(graph_store));

    // -- 2. NodeKindRegistry carrying the rubix-side kinds. The
    //       diag-render kind lives outside `starter.flow.*` so the
    //       public `register` path accepts it (R10).
    let kinds = NodeKindRegistry::new();
    let kind = KindId::new(DIAG_RENDER_KIND)
        .map_err(|e| anyhow::anyhow!("invalid kind id: {e}"))?;
    let behavior: Arc<dyn NodeBehavior> = Arc::new(DiagRenderNode {
        kind: kind.clone(),
    });
    kinds
        .register(behavior)
        .await
        .map_err(|e| anyhow::anyhow!("register diag-render kind: {e}"))?;

    // -- 3. FlowRegistry with the bundled scheduled-system-check
    //       flow. The body is built programmatically — the on-disk
    //       YAML at `rubix-flows/flows/` is the
    //       human-authored surface but its current shape predates
    //       the typed `FlowBody` projection and will be replaced
    //       once the `ai-agent` kind is wired into rubix-agent
    //       (later phase). For PR 3 the one-node renderer is
    //       enough to assert the FlowAsTool ↔ MCP locale path.
    let flow_id = FlowId::new(SCHEDULED_SYSTEM_CHECK_FLOW)
        .map_err(|e| anyhow::anyhow!("invalid flow id: {e}"))?;
    let render_node = NodeId::new(RENDER_NODE_ID)
        .map_err(|e| anyhow::anyhow!("invalid node id: {e}"))?;

    let mut node = NodeDecl::new(render_node.clone(), kind.clone());
    node.triggers = vec![SEED_SLOT.to_owned()];
    let mut body = FlowBody::new(flow_id.clone());
    body.nodes = vec![node];

    let revision = FlowRevisionId::new();
    let registry = Arc::new(FlowRegistry::new());

    let seed_slot = SlotRef::new(render_node.clone(), SEED_SLOT);
    let output_slot = SlotRef::new(render_node, OUTPUT_SLOT);

    let tool_id = KindId::new(SCHEDULED_SYSTEM_CHECK_FLOW)
        .map_err(|e| anyhow::anyhow!("invalid tool id: {e}"))?;

    let seed_slot_for_adapter = seed_slot.clone();
    let seed = Arc::new(move |_input: &Value| {
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
            "prefs": prefs,
            "percent": 89_i64,
            "free_bytes": 12_500_000_000_i64,
            "at_ms": 1_705_320_000_000_i64,
        });
        vec![(
            seed_slot_for_adapter.clone(),
            SlotValue::Json(payload),
        )]
    });

    let output_key = format!("{}.{}", output_slot.node, output_slot.slot);
    let output = Arc::new(move |out: &SlotMap| -> Value {
        match out.get(&output_key) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => Value::Null,
        }
    });

    let spec = FlowRegistration::new(
        body,
        revision,
        tool_id,
        SCHEDULED_SYSTEM_CHECK_FLOW,
        "Inspect rubix host health and alert if a threshold is crossed.",
    )
    .terminal_slots(vec![output_slot])
    .input_schema(json!({"type": "object"}))
    .output_schema(json!({
        "type": "object",
        "properties": {"rendered": {"type": "string"}},
        "required": ["rendered"]
    }))
    .with_adapters(seed, output);

    registry
        .register(spec, &kinds)
        .await
        .map_err(|e| anyhow::anyhow!("register scheduled-system-check: {e}"))?;

    Ok((registry, flow_id, revision, engine))
}

// ---------------------------------------------------------------------------
// Diag-render node body.
// ---------------------------------------------------------------------------

/// One-node body that renders a `rubix.system.disk.warn` diagnostic
/// in the caller's locale + timezone. The seed adapter snapshots
/// `starter_mcp::current_locale()` and the corresponding
/// [`ResolvedPreferences`] onto the input slot; this node looks
/// nothing up — the renderer runs purely on the payload, so
/// task-local propagation across the FlowRunner's spawn boundary is
/// a non-issue (R5 design note: the locale is captured at the seed-
/// adapter call site, where the with_locale scope is live).
struct DiagRenderNode {
    kind: KindId,
}

#[async_trait]
impl NodeBehavior for DiagRenderNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let payload = match input.get(SEED_SLOT) {
            Some(SlotValue::Json(v)) => v.clone(),
            _ => {
                return Err(NodeError::InvalidInput(
                    "diag-render: missing `payload` slot".to_owned(),
                ));
            }
        };

        let lang_str = payload.get("lang").and_then(|v| v.as_str()).unwrap_or("en");
        let lang = LanguageTag::parse(lang_str)
            .unwrap_or_else(|_| LanguageTag::parse("en").expect("'en' parses"));

        let prefs: ResolvedPreferences = serde_json::from_value(
            payload.get("prefs").cloned().unwrap_or(Value::Null),
        )
        .map_err(|e| NodeError::InvalidInput(format!("diag-render: prefs: {e}")))?;

        let percent = payload.get("percent").and_then(|v| v.as_i64()).unwrap_or(0);
        let free = payload
            .get("free_bytes")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let at_ms = payload.get("at_ms").and_then(|v| v.as_i64()).unwrap_or(0);

        let diag = Diagnostic::new(
            MessageKey::parse("rubix.system.disk.warn")
                .expect("hard-coded key parses"),
        )
        .with_param("percent", DiagnosticParam::I64(percent))
        .with_param("free", DiagnosticParam::I64(free))
        .with_param("at", DiagnosticParam::Timestamp(at_ms));

        let bundle = rubix_spi::i18n::rubix_bundle()
            .map_err(|e| NodeError::Backend(format!("rubix_bundle: {e}")))?;
        let rendered = bundle.render_diagnostic(&lang, &diag, &prefs);

        let mut out = SlotMap::new();
        out.insert(
            OUTPUT_SLOT.to_owned(),
            SlotValue::Json(json!({"rendered": rendered})),
        );
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Locale → ResolvedPreferences mapping.
// ---------------------------------------------------------------------------

/// Map a BCP-47 [`LanguageTag`] to a [`ResolvedPreferences`] whose
/// timezone, date format, time format, and language reflect a
/// reasonable default for the tag's region subtag.
///
/// PR 3 only needs the en-US and es-AR cases for the integration
/// test; the table is open for the remaining locales as goal-flows
/// land. Unknown tags fall through to the platform-default UTC /
/// ISO date / 24-hour clock posture.
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

// Touch the HashMap import so this file stays compatible with a
// future per-locale lookup table without re-touching the import
// block. (Empty placeholder — collapses to a no-op at codegen.)
const _: fn() = || {
    let _: HashMap<&'static str, &'static str> = HashMap::new();
};
