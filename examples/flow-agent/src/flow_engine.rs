//! Stage 2: real flow execution backed by `starter-flow`.
//!
//! Parses the stored UI `FlowGraph` JSON into a [`FlowTopology`] and
//! drives it through a [`FlowRunner`]. The host owns one shared
//! `StaticAiRunnerRegistry` (Claude registered under
//! `anthropic.claude`) and one empty `StaticToolRegistry`; the
//! `StaticTriggerChannelRegistry` is per-fire so each invocation gets
//! a fresh mpsc channel and a clean in-memory graph store (same
//! pattern `examples/notes/src/flow_demo.rs` uses for the codeless
//! demo).
//!
//! UI / engine kind & slot mapping
//! ------------------------------
//! The UI works in friendly names (`trigger`, `ai-agent`, `log`) with
//! short slot names. The engine wants reverse-DNS identifiers and the
//! built-in nodes' canonical slot names. The mapping is small and
//! lives in [`map_kind`] / [`map_slot`]:
//!
//! | UI kind      | Backend behavior          | UI slot → backend slot                     |
//! |--------------|---------------------------|--------------------------------------------|
//! | `trigger`    | [`TriggerExplicit`]       | `fire` → `payload`                         |
//! | `ai-agent`   | [`AiAgent`]               | `in` → `input`, `out` → `output`           |
//! | `log`        | [`Log`]                   | `value` → `value`, `emitted` → `emitted`   |
//!
//! Any other UI kind aborts the fire with `Invalid` (mapped to 422 in
//! the REST layer). Slot names not in the table pass through as-is so
//! the engine produces an actionable error if the user wires a slot
//! the backend node doesn't expose.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use starter_ai::runners::claude::ClaudeRunner;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{
    FlowRunner, FlowRunnerConfig, InMemoryRunStore, RunHandle, RunSpec,
    RunStore as EngineRunStore,
};
use starter_flow_nodes::ai_agent::{AgentInputKind, AiAgent, StaticAiRunnerRegistry};
use starter_flow_nodes::log::Log;
use starter_flow_nodes::tool_registry::{StaticToolRegistry, ToolRegistry};
use starter_flow_nodes::trigger_explicit::{
    StaticTriggerChannelRegistry, TriggerChannelRegistry, TriggerExplicit,
};
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId, RunId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};

/// Reverse-DNS provider id the Claude CLI runner registers under.
pub const CLAUDE_PROVIDER_ID: &str = "anthropic.claude";

/// Errors `FlowEngine::fire` can surface.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FlowEngineError {
    /// The graph JSON did not parse into the expected
    /// `{ nodes, edges }` shape.
    #[error("invalid flow graph: {0}")]
    Parse(#[from] serde_json::Error),
    /// The graph parsed but a semantic check failed (no trigger,
    /// unknown node kind, bad id, etc.).
    #[error("invalid flow graph: {0}")]
    Invalid(String),
    /// `FlowRunner::start` rejected the run (e.g. degraded engine).
    #[error("engine error: {0}")]
    Engine(String),
    /// Firing the trigger channel failed (every receiver dropped).
    #[error("trigger fire failed: {0}")]
    Fire(String),
}

/// Host-side flow engine, cloned cheaply.
#[derive(Clone)]
pub struct FlowEngine {
    inner: Arc<FlowEngineInner>,
}

struct FlowEngineInner {
    ai_runners: Arc<dyn AiRunnerRegistry>,
    tools: Arc<dyn ToolRegistry>,
    provider_id: KindId,
    quiescence: Duration,
}

impl FlowEngine {
    /// Construct the engine with the default registry set.
    ///
    /// Registers the Claude CLI runner under [`CLAUDE_PROVIDER_ID`].
    /// No tools are registered — the ai-agent body runs in
    /// `AgentInputKind::Cli` mode, where the CLI manages its own
    /// tool dispatch.
    pub fn new() -> Self {
        let provider_id =
            KindId::new(CLAUDE_PROVIDER_ID).expect("CLAUDE_PROVIDER_ID is reverse-DNS");

        let mut arr = StaticAiRunnerRegistry::new();
        arr.register(
            provider_id.clone(),
            Arc::new(ClaudeRunner) as Arc<dyn starter_spi::ai::AiRunner>,
        );
        let ai_runners: Arc<dyn AiRunnerRegistry> = Arc::new(arr);
        let tools: Arc<dyn ToolRegistry> = Arc::new(StaticToolRegistry::new());

        Self {
            inner: Arc::new(FlowEngineInner {
                ai_runners,
                tools,
                provider_id,
                quiescence: Duration::from_secs(60),
            }),
        }
    }

    /// Override the per-run quiescence window. The default is 60 s
    /// so Claude CLI invocations (5–30 s of silence on stdout while
    /// thinking) don't get reaped as idle. Tests that only exercise
    /// cheap nodes pass a shorter window (typically 200 ms) so the
    /// engine emits `RunCompleted` promptly after the terminal slot
    /// is written.
    pub fn with_quiescence(self, quiescence: Duration) -> Self {
        let inner = FlowEngineInner {
            ai_runners: self.inner.ai_runners.clone(),
            tools: self.inner.tools.clone(),
            provider_id: self.inner.provider_id.clone(),
            quiescence,
        };
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Borrow the AI runner registry (used by the agent-as-tool
    /// bridge in later stages).
    pub fn ai_runners(&self) -> &Arc<dyn AiRunnerRegistry> {
        &self.inner.ai_runners
    }

    /// Parse `graph_json` and fire the flow's trigger with `payload`.
    ///
    /// Returns the engine [`RunId`] and a [`RunHandle`]; the caller is
    /// expected to spawn an async task that drains
    /// `handle.initial_rx` until terminal and translates events onto
    /// the host's SSE bus. Parsing / materialisation errors surface
    /// as [`FlowEngineError::Invalid`] / [`FlowEngineError::Parse`]
    /// so the REST layer can map them to 422.
    pub async fn fire(
        &self,
        flow_db_id: &str,
        graph_json: &Value,
        payload: Value,
    ) -> Result<FireOutcome, FlowEngineError> {
        let parsed: UiFlowGraph = serde_json::from_value(graph_json.clone())?;

        // -----------------------------------------------------------
        // Identify the trigger node (and reject graphs without one).
        // -----------------------------------------------------------
        let trigger_nodes: Vec<&UiFlowNode> =
            parsed.nodes.iter().filter(|n| n.kind == "trigger").collect();
        if trigger_nodes.is_empty() {
            return Err(FlowEngineError::Invalid(
                "graph has no trigger node".to_owned(),
            ));
        }
        if trigger_nodes.len() > 1 {
            return Err(FlowEngineError::Invalid(format!(
                "graph has {} trigger nodes; expected exactly one",
                trigger_nodes.len()
            )));
        }
        let ui_trigger = trigger_nodes[0];

        // -----------------------------------------------------------
        // Build a per-fire trigger channel.
        // -----------------------------------------------------------
        let channel_id_str = sanitize_reverse_dns(&format!("flow-agent.channels.{flow_db_id}"));
        let channel_id = KindId::new(&channel_id_str).map_err(|e| {
            FlowEngineError::Invalid(format!("bad channel id `{channel_id_str}`: {e}"))
        })?;
        let mut tcr = StaticTriggerChannelRegistry::new();
        let sender = tcr.bind(channel_id.clone(), 16);
        let tcr_arc: Arc<dyn TriggerChannelRegistry> = Arc::new(tcr);

        // -----------------------------------------------------------
        // Materialise each UI node into a backend NodeBehavior.
        // -----------------------------------------------------------
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        let mut node_kinds: HashMap<String, String> = HashMap::new();
        let mut ui_to_engine: HashMap<String, NodeId> = HashMap::new();
        for n in &parsed.nodes {
            let engine_id_str = sanitize_reverse_dns(&format!("flow-agent.nodes.{}", n.id));
            let node_id = NodeId::new(&engine_id_str).map_err(|e| {
                FlowEngineError::Invalid(format!(
                    "bad engine id `{engine_id_str}` from ui id `{}`: {e}",
                    n.id
                ))
            })?;
            let behavior: Arc<dyn NodeBehavior> = match n.kind.as_str() {
                "trigger" => Arc::new(TriggerExplicit::new(tcr_arc.clone())),
                "ai-agent" => Arc::new(
                    AiAgent::new(self.inner.tools.clone(), self.inner.ai_runners.clone())
                        .with_provider_id(self.inner.provider_id.clone())
                        .with_input_kind(AgentInputKind::Cli),
                ),
                "log" => Arc::new(Log::new()),
                other => {
                    return Err(FlowEngineError::Invalid(format!(
                        "unsupported node kind: `{other}` (node `{}`)",
                        n.id
                    )));
                }
            };
            behaviors.insert(node_id.clone(), behavior);
            node_kinds.insert(n.id.clone(), n.kind.clone());
            ui_to_engine.insert(n.id.clone(), node_id);
        }

        let trigger_node_engine_id = ui_to_engine
            .get(&ui_trigger.id)
            .expect("trigger registered above")
            .clone();

        // -----------------------------------------------------------
        // Build the link map and a UI-edge index keyed on backend
        // slots for SSE `EdgeActive` lookup later.
        // -----------------------------------------------------------
        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        let mut edge_index: HashMap<(NodeId, String), Vec<String>> = HashMap::new();
        for e in &parsed.edges {
            let src_kind = node_kinds.get(&e.source).ok_or_else(|| {
                FlowEngineError::Invalid(format!("edge `{}` references unknown source node `{}`", e.id, e.source))
            })?;
            let tgt_kind = node_kinds.get(&e.target).ok_or_else(|| {
                FlowEngineError::Invalid(format!("edge `{}` references unknown target node `{}`", e.id, e.target))
            })?;
            let src_slot = map_slot(src_kind, &e.source_slot, SlotDirection::Output);
            let tgt_slot = map_slot(tgt_kind, &e.target_slot, SlotDirection::Input);

            let src_engine = ui_to_engine.get(&e.source).cloned().expect("indexed above");
            let tgt_engine = ui_to_engine.get(&e.target).cloned().expect("indexed above");

            links
                .entry(SlotRef::new(src_engine.clone(), src_slot.clone()))
                .or_default()
                .push(SlotRef::new(tgt_engine, tgt_slot));
            edge_index
                .entry((src_engine, src_slot))
                .or_default()
                .push(e.id.clone());
        }

        // -----------------------------------------------------------
        // Trigger slot lists per kind. The trigger node's seed
        // (`channel_id`) below kicks the run.
        // -----------------------------------------------------------
        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        for n in &parsed.nodes {
            let engine_id = ui_to_engine.get(&n.id).expect("indexed above").clone();
            let slots: Vec<&str> = match n.kind.as_str() {
                "trigger" => vec!["channel_id"],
                "ai-agent" => vec!["input"],
                "log" => vec!["value"],
                _ => vec![],
            };
            triggers.insert(engine_id, slots.into_iter().map(String::from).collect());
        }

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        // -----------------------------------------------------------
        // Build the FlowRunner. Fresh in-memory stores per fire — the
        // codeless demo's notes for why are at
        // examples/notes/src/flow_demo.rs::build_runner.
        // -----------------------------------------------------------
        let flow_id_str = sanitize_reverse_dns(&format!("flow-agent.flows.{flow_db_id}"));
        let flow_id = FlowId::new(&flow_id_str)
            .map_err(|e| FlowEngineError::Invalid(format!("bad flow id `{flow_id_str}`: {e}")))?;

        let seeds = vec![(
            SlotRef::new(trigger_node_engine_id.clone(), "channel_id"),
            SlotValue::String(channel_id.to_string()),
        )];
        let terminal_slots: Vec<SlotRef> = parsed
            .nodes
            .iter()
            .filter(|n| n.kind == "log")
            .map(|n| {
                SlotRef::new(
                    ui_to_engine.get(&n.id).expect("indexed above").clone(),
                    "emitted",
                )
            })
            .collect();

        let spec = RunSpec::new(
            flow_id,
            FlowRevisionId::new(),
            topology,
            seeds,
            terminal_slots,
        );

        let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn EngineRunStore> = Arc::new(InMemoryRunStore::new());
        let mut cfg = FlowRunnerConfig::default();
        // Claude CLI invocations can take 5–30s and produce no
        // SlotChanged events while in-flight — the configured
        // quiescence (defaults to 60 s) keeps the engine from reaping
        // the run as idle. Tests override via [`FlowEngine::with_quiescence`].
        cfg.quiescence = self.inner.quiescence;
        let runner = FlowRunner::new(graph_store, run_store).with_config(cfg);

        // Fire FIRST — the mpsc buffer absorbs the race between the
        // host send and the trigger body's recv (same reasoning as
        // notes/flow_demo).
        sender
            .fire(payload)
            .await
            .map_err(|e| FlowEngineError::Fire(e.to_string()))?;

        let handle = runner
            .start(spec, SlotMap::new())
            .await
            .map_err(|e| FlowEngineError::Engine(e.to_string()))?;

        let run_id = handle.run;
        Ok(FireOutcome {
            run_id,
            handle,
            edge_index,
            ui_to_engine,
        })
    }
}

impl Default for FlowEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// What `FlowEngine::fire` hands back to the REST handler.
pub struct FireOutcome {
    /// Engine run id (the host stores its own `Run` row keyed
    /// separately; the engine id is exposed for tracing).
    pub run_id: RunId,
    /// Live run handle — `initial_rx` is pre-subscribed and is
    /// guaranteed to see `RunStarted`.
    pub handle: RunHandle,
    /// Map from `(engine_node_id, backend_slot)` to the UI edge ids
    /// that fan out from that slot. Used by the SSE pump to emit
    /// `EdgeActive` events keyed on the UI's edge ids so the
    /// frontend can light the right wires.
    pub edge_index: HashMap<(NodeId, String), Vec<String>>,
    /// Map from UI node id → engine node id, so SSE node-status
    /// events can be reported back in the UI's id space.
    pub ui_to_engine: HashMap<String, NodeId>,
}

impl FireOutcome {
    /// Reverse lookup: engine node id → UI node id. O(n) but the
    /// graph is small.
    pub fn ui_node_id(&self, engine_id: &NodeId) -> Option<&str> {
        self.ui_to_engine
            .iter()
            .find_map(|(k, v)| (v == engine_id).then_some(k.as_str()))
    }
}

// ---------------------------------------------------------------------
// UI graph JSON shape (mirrors @nube/starter-ui-flow's FlowGraph).
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct UiFlowGraph {
    #[serde(default)]
    nodes: Vec<UiFlowNode>,
    #[serde(default)]
    edges: Vec<UiFlowEdge>,
}

#[derive(Debug, Deserialize)]
struct UiFlowNode {
    id: String,
    kind: String,
    // `position`, `label`, `data` are preserved by the store but the
    // engine doesn't need them.
}

#[derive(Debug, Deserialize)]
struct UiFlowEdge {
    id: String,
    source: String,
    #[serde(rename = "sourceSlot")]
    source_slot: String,
    target: String,
    #[serde(rename = "targetSlot")]
    target_slot: String,
}

// ---------------------------------------------------------------------
// Kind / slot mapping.
// ---------------------------------------------------------------------

#[derive(Clone, Copy)]
enum SlotDirection {
    Input,
    Output,
}

fn map_slot(ui_kind: &str, ui_slot: &str, dir: SlotDirection) -> String {
    match (ui_kind, ui_slot, dir) {
        ("trigger", "fire", SlotDirection::Output) => "payload".into(),
        ("ai-agent", "in", SlotDirection::Input) => "input".into(),
        ("ai-agent", "out", SlotDirection::Output) => "output".into(),
        // Fall through: assume the UI is already speaking the
        // backend's slot names (e.g. for `log`).
        _ => ui_slot.to_owned(),
    }
}

/// Massage an arbitrary string into a reverse-DNS-safe id. UI ids
/// like `trigger-abc12345` are single-segment and would fail the
/// engine's reverse-DNS validator; this wraps them under a fixed
/// prefix and replaces unsafe characters with `-`.
fn sanitize_reverse_dns(input: &str) -> String {
    fn sanitize_segment(seg: &str) -> String {
        let mut out = String::with_capacity(seg.len());
        for ch in seg.chars() {
            let ok = ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_';
            out.push(if ok {
                ch
            } else if ch.is_ascii_uppercase() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            });
        }
        // Reverse-DNS segments must start with `[a-z]`.
        if out.is_empty() || !out.chars().next().unwrap().is_ascii_lowercase() {
            out.insert(0, 'x');
        }
        out
    }
    input
        .split('.')
        .map(sanitize_segment)
        .collect::<Vec<_>>()
        .join(".")
}
