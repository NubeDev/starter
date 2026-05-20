//! Codeless-shape flow demo for the notes example (Phase 5
//! `starter-flow-phase5-demo` job).
//!
//! Wires a three-node flow — `trigger.explicit → ai-agent → log` —
//! against the local Claude Code CLI runner from `starter-ai`. A
//! `POST /api/flows/codeless-demo/fire` request supplies a prompt;
//! the handler fires the trigger, awaits run completion, and
//! returns the log node's passthrough output as JSON.
//!
//! Run locally:
//! ```sh
//! curl -X POST http://localhost:8080/api/flows/codeless-demo/fire \
//!   -H 'content-type: application/json' \
//!   -d '{"prompt": "say hi"}'
//! ```
//!
//! No API key needed — the Claude Code CLI runner uses whatever
//! authentication `claude auth login` configured locally. CI never
//! hits this endpoint; the end-to-end smoke at
//! `crates/smoke-tests/tests/codeless_shape_on_one_engine.rs`
//! drives the same topology against a `RecordingAiRunner`.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;

use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{FlowRunner, FlowRunnerConfig, InMemoryRunStore, RunSpec, RunStore};
use starter_flow_nodes::ai_agent::{AgentInputKind, AiAgent, StaticAiRunnerRegistry};
use starter_flow_nodes::log::Log;
use starter_flow_nodes::tool_registry::StaticToolRegistry;
use starter_flow_nodes::trigger_explicit::{
    StaticTriggerChannelRegistry, TriggerExplicit, TriggerSender,
};
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};

use starter_ai::runners::claude::ClaudeRunner;

/// Channel id the host binds the explicit trigger to.
const CHANNEL_ID: &str = "examples.notes.codeless-demo";
/// Reverse-DNS provider id under which the Claude runner registers.
const PROVIDER_ID: &str = "anthropic.claude";
/// Node ids inside the demo flow.
const TRIGGER_NODE: &str = "examples.notes.demo.trigger";
const AGENT_NODE: &str = "examples.notes.demo.agent";
const LOG_NODE: &str = "examples.notes.demo.log";
/// Flow id.
const FLOW_ID: &str = "examples.notes.codeless-demo";

/// Shared per-app demo state. Cloned cheaply (one `Arc` inside).
#[derive(Clone)]
pub struct FlowDemoState {
    inner: Arc<FlowDemoInner>,
}

struct FlowDemoInner {
    /// Frozen topology — same three nodes, same links, every run.
    topology: Arc<FlowTopology>,
    /// Sender half of the fire channel; cloned into each handler call.
    sender: TriggerSender,
    flow_id: FlowId,
    trigger_node: NodeId,
    log_node: NodeId,
}

impl FlowDemoState {
    /// Build the demo state: register the Claude runner under
    /// `anthropic.claude`, bind one fire channel, and assemble the
    /// three-node topology.
    pub fn build() -> anyhow::Result<Self> {
        let trigger_node = NodeId::new(TRIGGER_NODE)?;
        let agent_node = NodeId::new(AGENT_NODE)?;
        let log_node = NodeId::new(LOG_NODE)?;
        let channel_id = KindId::new(CHANNEL_ID)?;
        let provider_id = KindId::new(PROVIDER_ID)?;

        // Trigger channel registry + sender.
        let mut tcr = StaticTriggerChannelRegistry::new();
        let sender = tcr.bind(channel_id.clone(), 16);
        let tcr_arc: Arc<dyn starter_flow_nodes::trigger_explicit::TriggerChannelRegistry> =
            Arc::new(tcr);

        // AI runner registry — register the Claude CLI runner under
        // its reverse-DNS provider id.
        let mut arr = StaticAiRunnerRegistry::new();
        arr.register(
            provider_id.clone(),
            Arc::new(ClaudeRunner) as Arc<dyn starter_spi::ai::AiRunner>,
        );
        let arr_arc: Arc<dyn AiRunnerRegistry> = Arc::new(arr);

        // Tool registry — empty. The Claude CLI manages its own
        // tools internally; the ai-agent body's tool dispatch path
        // is unused on the Cli input path.
        let tools: Arc<dyn starter_flow_nodes::tool_registry::ToolRegistry> =
            Arc::new(StaticToolRegistry::new());

        // Node bodies.
        let trigger_body: Arc<dyn NodeBehavior> = Arc::new(TriggerExplicit::new(tcr_arc));
        let agent_body: Arc<dyn NodeBehavior> = Arc::new(
            AiAgent::new(tools, arr_arc)
                .with_provider_id(provider_id)
                .with_input_kind(AgentInputKind::Cli),
        );
        let log_body: Arc<dyn NodeBehavior> = Arc::new(Log::new());

        // Topology assembly.
        let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
        behaviors.insert(trigger_node.clone(), trigger_body);
        behaviors.insert(agent_node.clone(), agent_body);
        behaviors.insert(log_node.clone(), log_body);

        let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
        links.insert(
            SlotRef::new(trigger_node.clone(), "payload"),
            vec![SlotRef::new(agent_node.clone(), "input")],
        );
        links.insert(
            SlotRef::new(agent_node.clone(), "output"),
            vec![SlotRef::new(log_node.clone(), "value")],
        );

        let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
        triggers.insert(
            trigger_node.clone(),
            std::iter::once("channel_id".to_owned()).collect(),
        );
        triggers.insert(
            agent_node.clone(),
            std::iter::once("input".to_owned()).collect(),
        );
        triggers.insert(
            log_node.clone(),
            std::iter::once("value".to_owned()).collect(),
        );

        let topology = Arc::new(FlowTopology {
            links,
            triggers,
            behaviors,
        });

        Ok(Self {
            inner: Arc::new(FlowDemoInner {
                topology,
                sender,
                flow_id: FlowId::new(FLOW_ID)?,
                trigger_node,
                log_node,
            }),
        })
    }

    /// Build a fresh FlowRunner per fire so each run gets a clean
    /// InMemoryGraphStore. Sharing the store across runs hits the
    /// R3 idempotent-write short-circuit on the second seed
    /// (channel_id is already at the same value), so the trigger
    /// never re-fires. A per-run store sidesteps that and keeps
    /// the demo's "fire and see a fresh response" semantics. The
    /// future engine-wide solution is a per-run slot namespace;
    /// that's Phase 6 territory.
    fn build_runner() -> FlowRunner {
        let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
        let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
        let mut runner_config = FlowRunnerConfig::default();
        // Quiescence bumped to 60s — Claude CLI invocations can
        // take 5-30s, and the propagator emits no SlotChanged
        // events while a node body is mid-invoke. The handler
        // short-circuits on the log node's NodeEmitted so the
        // normal happy-path latency is ai-agent + ms, not 60s;
        // the long quiescence is the wall-clock failure bound.
        runner_config.quiescence = Duration::from_secs(60);
        FlowRunner::new(graph_store, run_store).with_config(runner_config)
    }

    /// Build the axum sub-router exposing the demo endpoint.
    /// Generic over the parent state `S` so the router composes into
    /// the notes app's `Router<AppState>` (or any other host's).
    pub fn router<S>(self) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        Router::new()
            .route("/api/flows/codeless-demo/fire", post(fire))
            .with_state(self)
    }
}

#[derive(Deserialize)]
struct FirePayload {
    /// The prompt the ai-agent node forwards to the Claude CLI.
    prompt: String,
}

#[derive(Serialize)]
struct FireResponse {
    /// Stringified [`RunId`] of the new run.
    run: String,
    /// Terminal `emitted` slot value of the log node, if the run
    /// completed within the timeout window. `None` on timeout /
    /// failure.
    log: Option<serde_json::Value>,
    /// Terminal [`RunStatus`] of the run, stringified.
    status: String,
}

/// `POST /api/flows/codeless-demo/fire`.
///
/// Fires the explicit trigger with the supplied prompt and awaits
/// the run's terminal status. Returns the log node's passthrough
/// `emitted` slot value as `log` so the caller sees what the AI
/// agent produced.
async fn fire(
    State(state): State<FlowDemoState>,
    Json(body): Json<FirePayload>,
) -> Result<Json<FireResponse>, (StatusCode, String)> {
    let inner = state.inner.clone();

    let seeds = vec![(
        SlotRef::new(inner.trigger_node.clone(), "channel_id"),
        SlotValue::String(CHANNEL_ID.to_owned()),
    )];
    let terminal_slots = vec![SlotRef::new(inner.log_node.clone(), "emitted")];

    let spec = RunSpec::new(
        inner.flow_id.clone(),
        FlowRevisionId::new(),
        inner.topology.clone(),
        seeds,
        terminal_slots,
    );

    // Fire FIRST — the mpsc buffer absorbs the race between the
    // host send and the trigger body's recv. This way we never miss
    // a payload even if the run hasn't reached the trigger node
    // yet.
    // ai-agent's `input` slot reads a string via read_string —
    // it accepts SlotValue::String or SlotValue::Json(Value::String),
    // but not a JSON object. Fire the bare prompt string.
    inner
        .sender
        .fire(serde_json::Value::String(body.prompt.clone()))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("fire failed: {e}"),
            )
        })?;

    let runner = FlowDemoState::build_runner();
    let mut handle = runner.start(spec, SlotMap::new()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("start failed: {e}"),
        )
    })?;

    let run_id = handle.run.to_string();

    // Drain events until RunCompleted / RunFailed / RunCancelled.
    // Bound the wait — the Claude CLI can take a while but the
    // demo handler shouldn't block a request thread indefinitely.
    let outcome = timeout(
        Duration::from_secs(120),
        drain_to_terminal(&mut handle, &inner.log_node),
    )
    .await
    .map_err(|_| {
        (
            StatusCode::REQUEST_TIMEOUT,
            "flow run did not complete within 120s".to_owned(),
        )
    })?;

    let (status, output) = outcome;
    // SlotMap keys in RunCompleted.output are `<node_id>.<slot>` —
    // not just the bare slot name. (See e.g. the stage-5 smoke
    // assertion at crates/smoke-tests/tests/codeless_shape_on_one_engine.rs.)
    let emitted_key = format!("{LOG_NODE}.{}", "emitted");
    let log = output
        .as_ref()
        .and_then(|o| o.get(&emitted_key))
        .map(slot_to_json);

    Ok(Json(FireResponse {
        run: run_id,
        log,
        status,
    }))
}

async fn drain_to_terminal(
    handle: &mut starter_flow::run::RunHandle,
    log_node: &NodeId,
) -> (String, Option<SlotMap>) {
    use tokio::sync::broadcast::error::RecvError;
    loop {
        match handle.initial_rx.recv().await {
            Ok(FlowEvent::RunCompleted { output, .. }) => {
                return ("Completed".to_owned(), Some(output));
            }
            Ok(FlowEvent::RunFailed { error, .. }) => {
                return (format!("Failed: {error}"), None);
            }
            Ok(FlowEvent::RunCancelled { .. }) => {
                return ("Cancelled".to_owned(), None);
            }
            // Short-circuit on the terminal log node's `emitted`
            // emit so the handler returns immediately instead of
            // waiting out the full quiescence window (which can
            // be tens of seconds because a node mid-invoke
            // produces no SlotChanged events).
            Ok(FlowEvent::NodeEmitted {
                node, slot, value, ..
            }) if &node == log_node && slot == "emitted" => {
                let mut out = SlotMap::new();
                out.insert(format!("{node}.{slot}"), value);
                return ("Completed".to_owned(), Some(out));
            }
            Ok(_) => {}
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => {
                return ("Closed".to_owned(), None);
            }
        }
    }
}

fn slot_to_json(v: &SlotValue) -> serde_json::Value {
    match v {
        SlotValue::String(s) => serde_json::Value::String(s.clone()),
        SlotValue::Json(j) => j.clone(),
        SlotValue::Int(i) => serde_json::Value::from(*i),
        SlotValue::Bool(b) => serde_json::Value::Bool(*b),
        other => serde_json::Value::String(format!("{other:?}")),
    }
}

/// `impl IntoResponse` for the error tuple to keep the handler
/// signature tidy.
#[allow(dead_code)]
fn _err(status: StatusCode, msg: String) -> impl IntoResponse {
    (status, msg)
}
