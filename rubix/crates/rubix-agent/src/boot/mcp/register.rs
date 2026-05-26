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

use starter_ext_host::ExtensionRegistry;
use starter_flow::definition::body::FlowBody;
use starter_flow::engine::Engine;
use starter_flow::registry::NodeKindRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_surfaces::{FlowRegistration, FlowRegistry};

use starter_spi::ai::AiRunner;
use starter_spi::i18n::LanguageTag;
use starter_spi::tool::Tool;
use starter_store_postgres::pool::Pool;
use starter_store_warehouse::ChClient;

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
    extensions: Option<&ExtensionRegistry>,
    shared_tools: Option<Vec<Arc<dyn Tool>>>,
) -> anyhow::Result<(
    Arc<FlowRegistry>,
    Vec<(FlowId, FlowRevisionId)>,
    Arc<Engine>,
)> {
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
    let runner: Arc<dyn AiRunner> =
        ai::build_runner(&cfg).map_err(|e| anyhow::anyhow!("boot::ai::build_runner: {e}"))?;
    // Reuse the caller-supplied tool registry when present so the
    // MCP-side `ai-agent` node dispatches to the SAME `Arc<dyn Tool>`
    // instances the REST `/api/v1/tools/*` router serves. Without
    // this share, every backed-by-`InMemory*Store` tool family
    // (dashboards, users, etc.) gets a second store the REST writes
    // never reach. The laptop / no-shared path (the stdio
    // `rubix-admin mcp` subcommand) falls back to rebuilding the
    // registry locally so it keeps working standalone.
    let tool_registry_snapshot: Vec<Arc<dyn Tool>> = match shared_tools {
        Some(tools) => tools,
        None => crate::registry::build_tool_registry(
            ch_client,
            cfg.insights.disk_warn_threshold,
            pg_pool.clone(),
            cfg.blob_root.clone(),
        ),
    };

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
        tracing::info!("no Postgres pool — loading flow definitions from the embedded bundle",);
        rubix_flows::load_all().map_err(|e| anyhow::anyhow!("rubix_flows::load_all: {e}"))?
    };

    let kinds = NodeKindRegistry::new();
    let ai_agent_kind = KindId::new(AI_AGENT_KIND_ID)
        .map_err(|e| anyhow::anyhow!("invalid {AI_AGENT_KIND_ID} kind id: {e}"))?;
    // Snapshot the MCP service wiring once per boot. The agent
    // node passes these into `CliCfg` for every narration call so
    // the Claude wrapper attaches to the rubix `/api/v1/mcp`
    // bridge and the model can dispatch host tools mid-turn
    // (D-F5.6). Both unset = narration only (the legacy
    // catalogue-less behaviour); see
    // `rubix/docs/sessions/2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md`
    // §"Part 3" for why this is opt-in via env vars rather than
    // auto-derived from the bind address — a follow-up will bake
    // a service-token mechanism so operators do not have to
    // copy-paste a bearer.
    let mcp_url = std::env::var("RUBIX_SERVICE_MCP_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let mcp_token = std::env::var("RUBIX_SERVICE_MCP_TOKEN")
        .ok()
        .filter(|s| !s.trim().is_empty());
    // Clone the snapshot before handing it to the ai-agent node so
    // the tool-call kind registration below can share the same
    // `Arc<dyn Tool>` instances. Cheap — each entry is an Arc.
    let tool_call_snapshot_for_kind = tool_registry_snapshot.clone();
    let ai_node: Arc<dyn NodeBehavior> = Arc::new(RubixAiAgentNode::new(
        ai_agent_kind.clone(),
        runner,
        tool_registry_snapshot,
        mcp_url,
        mcp_token,
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
    // rubix flows reference (counter, log, trigger.schedule,
    // tool-call). Without these, flows like `com.rubix.tick-counter`
    // (added in PR #38) fail topology resolution at boot with
    // `unknown node kind starter.flow.trigger.schedule`. The kind
    // set must stay in sync with `crate::registry::builtin_kind_behaviors`,
    // which populates the `rubix.flow_ops.kinds` listing.
    //
    // `tool-call` reuses the same `tool_registry_snapshot` the
    // ai-agent node was constructed with so REST writes, agent-
    // dispatched tool calls, and flow-dispatched tool calls all
    // share one `Arc<dyn Tool>` per id (avoids per-surface in-memory
    // store divergence). See rubix/docs/sessions/data-flow/01-producer.md
    // for the framework split this enables.
    let mut tool_call_registry = starter_flow_nodes::tool_registry::StaticToolRegistry::new();
    for tool in &tool_call_snapshot_for_kind {
        let id = tool.definition().name;
        match KindId::new(id.clone()) {
            Ok(kid) => tool_call_registry.register(kid, tool.clone()),
            Err(e) => tracing::warn!(
                tool_id = %id,
                error = %e,
                "tool id is not a valid reverse-DNS KindId; skipping tool-call binding",
            ),
        }
    }
    let tool_call_registry: Arc<dyn starter_flow_nodes::tool_registry::ToolRegistry> =
        Arc::new(tool_call_registry);

    for kind in [
        Arc::new(starter_flow_nodes::counter::Counter::new()) as Arc<dyn NodeBehavior>,
        Arc::new(starter_flow_nodes::log::Log::new()) as Arc<dyn NodeBehavior>,
        Arc::new(starter_flow_nodes::trigger_schedule::TriggerSchedule::new())
            as Arc<dyn NodeBehavior>,
        Arc::new(starter_flow_nodes::tool_call::ToolCall::new(
            tool_call_registry.clone(),
        )) as Arc<dyn NodeBehavior>,
    ] {
        let kind_id = kind.kind_id().to_string();
        kinds
            .register_builtin(kind)
            .await
            .map_err(|e| anyhow::anyhow!("register `{kind_id}`: {e}"))?;
    }

    // -- 3. Extension-contributed node kinds. Pure composition over
    //       upstream `starter-ext-flow`; rubix owns no walker logic
    //       (SCOPE R2). Slice A binds the upstream placeholder
    //       behaviour. A failure to register one bundle does not abort
    //       boot — log and continue so the remaining flows still load.
    if let Some(ext_registry) = extensions {
        match crate::boot::register_contributed_nodes(ext_registry, &kinds).await {
            Ok(n) if n > 0 => tracing::info!(
                target: "rubix.boot.extensions.flow",
                contributed_node_kinds = n,
                "extension-contributed node kinds registered"
            ),
            Ok(_) => {}
            Err(e) => tracing::warn!(
                target: "rubix.boot.extensions.flow",
                error = %e,
                "failed to register one or more extension-contributed node kinds; continuing"
            ),
        }
    }

    // -- 4. FlowRegistry seeded from every bundled YAML.
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

/// Read the SKILL.md body for a rubix bundled skill, stripping the
/// YAML frontmatter so what remains is the actual playbook the
/// model should be primed with. Returns `None` when the hint does
/// not match a bundled skill, when the file is missing, or when
/// the body after the frontmatter is empty.
///
/// Today the bundled-skill id (`com.rubix.<name>`) maps 1:1 to the
/// directory layout exposed by [`rubix_skills::bundled()`]
/// (`<name>/SKILL.md`); a future starter-skills migration can
/// replace this lookup with `SkillRegistry::get(...)` without
/// changing the seed-adapter call site.
fn skill_body_for_hint(hint: &str) -> Option<String> {
    // Strip the reverse-DNS namespace prefix (`com.rubix.`) to get
    // the directory name; anything else is not a rubix bundled
    // skill so we cannot resolve it from this crate.
    let name = hint.strip_prefix("com.rubix.")?;
    let dir = rubix_skills::bundled();
    let file = dir.get_file(format!("{name}/SKILL.md"))?;
    let raw = file.contents_utf8()?;
    let body = strip_frontmatter(raw).trim();
    if body.is_empty() {
        None
    } else {
        Some(body.to_owned())
    }
}

/// Strip a leading YAML frontmatter block (delimited by `---`
/// lines) from a SKILL.md source. Returns the slice unchanged when
/// no frontmatter is present; returns the body after the closing
/// fence otherwise. Matches the loader convention documented in
/// `rubix/docs/design/skills/README.md`.
fn strip_frontmatter(src: &str) -> &str {
    let rest = match src.strip_prefix("---\n") {
        Some(rest) => rest,
        None => return src,
    };
    match rest.find("\n---\n") {
        Some(end) => &rest[end + "\n---\n".len()..],
        // Unterminated frontmatter — degrade gracefully, return
        // everything after the opening fence rather than the
        // original (which would feed `---\n…` straight to the
        // model).
        None => rest,
    }
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

    // Per-flow CLI built-in restriction, captured at register time.
    // Surface contract: `Some(Vec<String>)` when the root node
    // declares `config.tools` in YAML (empty Vec = "MCP only, no
    // built-ins"); `None` when the key is absent (CLI default
    // catalogue stays in scope). The seed adapter forwards this
    // verbatim so the shared agent node body can hand it to
    // `AgentLoop::with_cli_tools`. See stage 07 § "lock down the
    // tool surface".
    let cli_tools: Option<Vec<String>> = body
        .nodes
        .first()
        .and_then(|n| n.settings.get(rubix_flows::yaml::TOOLS_KEY))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        });

    // Per-flow skill body, captured at register time. The seed
    // adapter embeds this into the seed payload so the agent node
    // can prepend it to the LLM prompt as a system-style preamble.
    // Until the upstream `AiAgent::with_kind_id` swap lands
    // (rubix/docs/sessions/2026-05-25-dashboards-sidebar-sse-and-chat-gaps.md
    // step 1+2), this is the only place the dashboard-builder /
    // system-checker / … playbook actually reaches the model. The
    // hint comes from the flow YAML's root-node `config.skill_hint`
    // — see e.g. `flows/dashboard-assistant.yaml`. None when the
    // hint is missing or unresolvable.
    let skill_body = body
        .nodes
        .first()
        .and_then(|n| n.settings.get("skill_hint"))
        .and_then(|v| v.as_str())
        .and_then(skill_body_for_hint);

    // If the root node is a `trigger.schedule`, project its
    // `settings.cron_expr` into the runtime `cron_expr` input slot
    // it expects. Until TopologyResolver/HR5 lands the projection
    // automatically, the surface has to seed it; without this the
    // scheduled dispatch of e.g. `com.rubix.tick-counter` fails
    // per tick with `trigger.schedule input missing cron_expr
    // slot`.
    let trigger_cron_seed: Option<(SlotRef, String)> =
        if root.kind.as_str() == starter_flow_nodes::trigger_schedule::KIND_ID {
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

    // Project each `starter.flow.tool-call` node's
    // `settings.tool_id` / `settings.tool_input` into the runtime
    // input slots its body reads. Same shape as the `cron_expr`
    // projection above — until TopologyResolver/HR5 lands, the
    // surface seeds these per fire so flows like
    // `com.rubix.data-flow.producer` can declaratively name the
    // tool to invoke from YAML rather than from an upstream link.
    //
    // `tick_epoch_ms` is auto-injected into the tool_input when the
    // YAML omits it AND the input is a JSON object — this is the
    // bridge from the schedule fire's wall-clock to the synth tool.
    // Tool authors can override by setting `tick_epoch_ms` explicitly
    // in YAML.
    //
    // **Link-driven `input` (B2 fix, 2026-05-26):** when the body
    // declares any link writing into `<node>.input`, the YAML
    // `settings.tool_input` is treated as a *default only* — the
    // seed adapter skips the per-fire projection so the link's
    // payload reaches the body without racing with the YAML seed.
    // The YAML projection still wins for `<node>.tool_id` (no
    // upstream link supplies it today). See
    // `rubix/docs/sessions/data-flow/02-ingest-l1-blockers-2026-05-26.md`.
    //
    // Note (multi-fire fix, 2026-05-26): `tool_id` and `input` are
    // declared as *read* slots by the `tool-call` kind
    // (`NodeBehavior::read_slots`), not triggers. Re-writing them
    // here per invoke therefore does **not** wake the node — only the
    // upstream link (`tick.fire → synth.in`) does. The `tool_id`
    // write is effectively a no-op under R3's idempotent
    // short-circuit (same value, `force = false`), and the
    // `tool_input` write differs each fire by `tick_epoch_ms` but
    // still triggers no extra invocation. See
    // `rubix/docs/sessions/data-flow/2026-05-26-data-flow-01-producer-multi-fire-root-cause.md`.
    let tool_call_seeds: Vec<(NodeId, String, Value, bool)> = body
        .nodes
        .iter()
        .filter(|n| n.kind.as_str() == starter_flow_nodes::tool_call::KIND_ID)
        .filter_map(|n| {
            let tool_id = n.settings.get("tool_id").and_then(|v| v.as_str())?;
            let tool_input = n
                .settings
                .get("tool_input")
                .cloned()
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            let input_link_target = format!(
                "{}.{}",
                n.id,
                starter_flow_nodes::tool_call::TOOL_INPUT_SLOT,
            );
            let input_link_driven = body.links.iter().any(|l| l.to == input_link_target);
            Some((
                n.id.clone(),
                tool_id.to_owned(),
                tool_input,
                input_link_driven,
            ))
        })
        .collect();

    let seed_slot_for_adapter = seed_slot.clone();
    let primary_tool_for_adapter = primary_tool.clone();
    let skill_body_for_adapter = skill_body.clone();
    let cli_tools_for_adapter = cli_tools.clone();
    let trigger_cron_for_adapter = trigger_cron_seed.clone();
    let tool_call_seeds_for_adapter = tool_call_seeds.clone();
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
        // Capture the principal **here**, on the request task,
        // before the engine spawns the flow run on a fresh task
        // that does not inherit `starter_mcp::PRINCIPAL`. Without
        // this snapshot the node body would always see
        // `current_principal() == None` and the dashboard write
        // verbs would 400 with `missing field tenant_id`. We
        // enrich the `input` payload only — `or_insert`-style so
        // explicit MCP `arguments` keep overriding session
        // defaults — and fall back to `DEFAULT_TENANT` when the
        // session is unbound (laptop dev path).
        let enriched_input = enrich_input_with_principal(input.clone());
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
            "input": enriched_input,
            "primary_tool": primary_tool_for_adapter,
            "skill_body": skill_body_for_adapter,
            "cli_tools": cli_tools_for_adapter,
            "_nonce": nonce.to_string(),
        });
        // Wall-clock for tool_input.tick_epoch_ms auto-injection.
        let now_epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let tool_call_writes = tool_call_seeds_for_adapter.iter().flat_map(
            |(node_id, tool_id, tool_input, input_link_driven)| {
                let mut writes = vec![(
                    SlotRef::new(node_id.clone(), starter_flow_nodes::tool_call::TOOL_ID_SLOT),
                    SlotValue::String(tool_id.clone()),
                )];
                if !*input_link_driven {
                    let mut input = tool_input.clone();
                    if let Value::Object(ref mut map) = input {
                        map.entry("tick_epoch_ms".to_owned())
                            .or_insert_with(|| json!(now_epoch_ms));
                    }
                    writes.push((
                        SlotRef::new(
                            node_id.clone(),
                            starter_flow_nodes::tool_call::TOOL_INPUT_SLOT,
                        ),
                        SlotValue::Json(input),
                    ));
                }
                writes
            },
        );
        vec![(seed_slot_for_adapter.clone(), SlotValue::Json(payload))]
            .into_iter()
            .chain(
                trigger_cron_for_adapter
                    .clone()
                    .map(|(slot, cron)| (slot, SlotValue::String(cron))),
            )
            .chain(tool_call_writes)
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

/// Conventional tenant id used when the session principal carries
/// no tenant binding (current rubix login default — see
/// `crates/starter-auth-users/src/routes/login.rs`). Matches the
/// constant of the same name in `boot::mcp::agent_node` and in
/// `routes::dashboard_events`; all three move together when real
/// tenant binding lands.
const DEFAULT_TENANT: &str = "system";

/// Merge the request-task principal (tenant / subject) into a
/// flow's caller `input` payload. Called from the seed adapter so
/// it executes on the request task — where
/// `starter_mcp::current_principal()` is still bound — rather than
/// inside the spawned flow run task. Caller-supplied keys win;
/// missing keys are filled from the principal, with a final
/// fallback to [`DEFAULT_TENANT`] for the `tenant_id` field when
/// the session is unbound.
fn enrich_input_with_principal(input: Value) -> Value {
    let principal = starter_mcp::current_principal();
    let mut value = input;
    let Some(obj) = value.as_object_mut() else {
        return value;
    };
    if !obj.contains_key("tenant_id") {
        let tid = principal
            .as_ref()
            .and_then(|p| p.tenant_id.as_deref())
            .unwrap_or(DEFAULT_TENANT);
        obj.insert("tenant_id".to_owned(), json!(tid));
    }
    if let Some(subject) = principal.as_ref().map(|p| p.subject.as_str()) {
        if !subject.is_empty() {
            obj.entry("owner_principal".to_owned())
                .or_insert_with(|| json!(subject));
            obj.entry("created_by".to_owned())
                .or_insert_with(|| json!(subject));
        }
    }
    value
}
