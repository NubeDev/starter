//! Flow-registry assembly for the rubix MCP surface.
//!
//! Loads every bundled flow YAML, registers the `com.rubix.ai-agent`
//! node kind backed by [`super::agent_node::RubixAiAgentNode`], and
//! wires each flow's seed/output adapter pair so the MCP
//! `tools/call` dispatch produces a structured `Diagnostic` response
//! per [docs/design/i18n-prefs/](../../../../docs/design/i18n-prefs/README.md)
//! and [docs/design/flows/](../../../../docs/design/flows/README.md).
//!
//! The seed adapter is where the U1 contract lives: the closure runs
//! inside the `starter-mcp` task-local that holds the caller's
//! [`LanguageTag`]; this module never re-parses
//! `Accept-Language`/`_meta.acceptLanguage` and never threads a
//! `LanguageTag` through call sites by hand.

use std::sync::Arc;

use serde_json::{json, Value};

use rubix_flows::AI_AGENT_KIND_ID;

use starter_flow::definition::body::FlowBody;
use starter_flow::engine::Engine;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, NodeBehavior, SlotMap, SlotRef, SlotValue};
use starter_flow_surfaces::{FlowRegistration, FlowRegistry};

use starter_spi::ai::AiRunner;
use starter_spi::i18n::LanguageTag;
use starter_spi::tool::Tool;
use starter_store_clickhouse::ChClient;
use starter_store_postgres::pool::Pool;

use crate::boot::ai;
use crate::boot::config::AgentConfig;
use crate::boot::flows_seed;

use super::agent_node::RubixAiAgentNode;
use super::prefs::prefs_from_locale;

/// Lower-level entry point exposed so integration tests can drive
/// the same wiring without standing up the HTTP listener. Returns
/// the registry, the list of `(flow_id, revision)` pairs that were
/// landed, and an [`Arc<Engine>`](Engine) bound to a fresh in-memory
/// graph store.
///
/// `ch_client` is the same `Arc<ChClient>` the binary's REST tool
/// composition threads into [`crate::registry::build_tool_registry`].
/// Passing it here means tool dispatch through the MCP / `ai-agent`
/// path persists `system_disk_history` rows to the warehouse just
/// like the REST path does. `None` is the laptop / no-warehouse
/// path — history writes are skipped per the
/// `DiskTool::with_history` contract.
pub async fn build_flow_registry(
    ch_client: Option<Arc<ChClient>>,
    pg_pool: Option<Pool>,
    runtime: Option<&crate::boot::FlowRuntime>,
) -> anyhow::Result<(Arc<FlowRegistry>, Vec<(FlowId, FlowRevisionId)>, Arc<Engine>)> {
    // -- 1. Engine on a fresh in-memory graph store. The terminal-
    //       slot read-back in `FlowAsTool` reads through this store.
    let engine = super::build_engine(runtime);

    // -- 2. NodeKindRegistry carrying the real `ai-agent` behaviour.
    //       Build the host's `AiRunner` per `boot::ai::build_runner`
    //       (Claude CLI by default; fixture-replay when
    //       `RUBIX_AI_FIXTURE` is set), snapshot the rubix tool list
    //       so the loop can dispatch model-requested tools, and
    //       register a thin wrapper around starter-flow-node-loop's
    //       `AiAgentNode` whose `kind_id()` matches the rubix flow
    //       YAML (`com.rubix.ai-agent`).
    let cfg = AgentConfig::load().unwrap_or_default();
    let runner: Arc<dyn AiRunner> = ai::build_runner(&cfg)
        .map_err(|e| anyhow::anyhow!("boot::ai::build_runner: {e}"))?;
    let tool_registry_snapshot: Vec<Arc<dyn Tool>> =
        crate::registry::build_tool_registry(ch_client, cfg.insights.disk_warn_threshold);

    // Source the flow definitions from PG when a pool is wired
    // in — that is the Phase D contract: PG is the source of
    // truth, the bundled YAMLs are only the first-boot seed.
    // The laptop / no-Postgres path falls back to the embedded
    // bundle directly so `cargo run -p rubix-agent` still works
    // without a database.
    let triples = if let Some(pool) = pg_pool.as_ref() {
        let (rows, inserted) = flows_seed::seed_and_load(pool)
            .await
            .map_err(|e| anyhow::anyhow!("flows_seed::seed_and_load: {e}"))?;
        tracing::info!(
            inserted,
            loaded = rows.len(),
            "flows_definitions sourced from Postgres",
        );
        rows
    } else {
        tracing::info!(
            "no Postgres pool — loading flow definitions from the embedded bundle",
        );
        rubix_flows::load_all()
            .map_err(|e| anyhow::anyhow!("rubix_flows::load_all: {e}"))?
    };

    let kinds = NodeKindRegistry::new();
    let ai_agent_kind = KindId::new(AI_AGENT_KIND_ID)
        .map_err(|e| anyhow::anyhow!("invalid {AI_AGENT_KIND_ID} kind id: {e}"))?;
    let ai_node: Arc<dyn NodeBehavior> = Arc::new(RubixAiAgentNode::new(
        ai_agent_kind.clone(),
        runner,
        tool_registry_snapshot,
    ));
    kinds
        .register(ai_node)
        .await
        .map_err(|e| anyhow::anyhow!("register {AI_AGENT_KIND_ID}: {e}"))?;
    tracing::info!(
        node_kinds = AI_AGENT_KIND_ID,
        "rubix ai-agent node kind registered"
    );

    // Register the built-in starter-flow node kinds the bundled
    // rubix flows reference (counter, log, trigger.schedule). Without
    // these, flows like `com.rubix.tick-counter` (added in PR #38)
    // fail topology resolution at boot with `unknown node kind
    // starter.flow.trigger.schedule`. The kind set must stay in
    // sync with `crate::registry::builtin_kind_behaviors`, which
    // populates the `rubix.flow_ops.kinds` listing.
    for kind in [
        Arc::new(starter_flow_nodes::counter::Counter::new()) as Arc<dyn NodeBehavior>,
        Arc::new(starter_flow_nodes::log::Log::new()) as Arc<dyn NodeBehavior>,
        Arc::new(starter_flow_nodes::trigger_schedule::TriggerSchedule::new())
            as Arc<dyn NodeBehavior>,
    ] {
        let kind_id = kind.kind_id().to_string();
        kinds
            .register_builtin(kind)
            .await
            .map_err(|e| anyhow::anyhow!("register `{kind_id}`: {e}"))?;
    }

    // -- 3. FlowRegistry seeded from every bundled YAML.
    let registry = Arc::new(FlowRegistry::new());
    let mut flows = Vec::new();
    for (flow_id, revision, body) in triples {
        register_one(&registry, &kinds, &flow_id, revision, body).await?;
        flows.push((flow_id, revision));
    }

    Ok((registry, flows, engine))
}

/// Extract the root node's first `allowed_tools` entry — the
/// concrete `Tool` the [`RubixAiAgentNode`] dispatches when this
/// flow runs. Returns `None` if the flow's root node has no
/// `allowed_tools` setting, in which case the agent node falls
/// back to the agent-loop-reply-only path.
///
/// We must key per-flow (not per-`NodeId`) because every rubix
/// flow's root node uses the same id (`agent` / `check`) so a
/// `HashMap<NodeId, String>` collides — last-seeded would win,
/// dispatching the wrong primary tool. The per-flow seed adapter
/// captures this value into the seed payload and the node body
/// reads it back from there.
fn primary_tool_for_root(body: &FlowBody) -> Option<String> {
    let root = body.nodes.first()?;
    let arr = root.settings.get("allowed_tools")?.as_array()?;
    let first = arr.first()?.as_str()?;
    Some(first.to_owned())
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

    // Per-flow primary tool, captured at register time. The seed
    // closure embeds this into the seed-slot JSON payload so the
    // (shared) `com.rubix.ai-agent` node body can read which tool
    // to dispatch on a per-invocation basis instead of via a
    // NodeId-keyed lookup (node ids collide across flows).
    let primary_tool = primary_tool_for_root(&body);

    // If the root node is a `trigger.schedule`, project its
    // `settings.cron_expr` into the runtime `cron_expr` input slot
    // it expects. Until TopologyResolver/HR5 lands the projection
    // automatically, the surface has to seed it; without this the
    // scheduled dispatch of e.g. `com.rubix.tick-counter` fails
    // per tick with `trigger.schedule input missing cron_expr
    // slot`.
    let trigger_cron_seed: Option<(SlotRef, String)> = if root.kind.as_str()
        == starter_flow_nodes::trigger_schedule::KIND_ID
    {
        root.settings
            .get("cron_expr")
            .and_then(|v| v.as_str())
            .map(|s| {
                (
                    SlotRef::new(
                        root.id.clone(),
                        starter_flow_nodes::trigger_schedule::CRON_EXPR_SLOT,
                    ),
                    s.to_owned(),
                )
            })
    } else {
        None
    };

    let seed_slot_for_adapter = seed_slot.clone();
    let primary_tool_for_adapter = primary_tool.clone();
    let trigger_cron_for_adapter = trigger_cron_seed.clone();
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
        // A per-call nonce keeps each seed write distinct from the
        // last so the engine slot store cannot deduplicate one run
        // against another (every FlowAsTool::invoke is meant to be a
        // fresh RPC). `SystemTime` is enough resolution because the
        // engine resolves seeds, not nonce values; the field is just
        // ballast so equality checks on the payload differ.
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let payload = json!({
            "lang": prefs.language,
            "locale": lang.as_str(),
            "prefs": prefs,
            "input": input.clone(),
            "primary_tool": primary_tool_for_adapter,
            "_nonce": nonce.to_string(),
        });
        vec![(seed_slot_for_adapter.clone(), SlotValue::Json(payload))]
            .into_iter()
            .chain(
                trigger_cron_for_adapter
                    .clone()
                    .map(|(slot, cron)| (slot, SlotValue::String(cron))),
            )
            .collect()
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

