//! Phase 5 stage 5 — SCOPE smoke: "Codeless shape on one engine."
//!
//! Asserts the end-to-end three-node chain (`trigger.explicit →
//! ai-agent → log`) propagates correctly on one engine:
//!
//! 1. `RecordingAiRunner` saw exactly one call, proving the trigger
//!    payload reached the ai-agent.
//! 2. The log node's passthrough `emitted` terminal slot carries the
//!    ai-agent's scripted reply, proving the agent's `output` slot
//!    reached the log node's `value` input through the propagator.
//!
//! Proves R1 (everything is a node), R2 (one write chokepoint), and
//! the codeless-shape propagator routing without needing the real
//! Claude CLI runner. The notes host wires the same three-node
//! topology against `ClaudeRunner` in
//! `examples/notes/src/flow_demo.rs`; CI runs only this smoke.
//!
//! The `starter.flow.log` tracing event the log body emits is
//! covered by that body's unit tests; cargo test's per-process
//! tracing subscriber doesn't survive the propagator's worker-thread
//! boundary so we don't re-assert it here (same precedent as
//! `ai_agent_is_just_a_node_kind.rs` lines 130-136).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use starter_ai::testing::{RecordingAiRunner, ScriptTurn};
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{FlowRunner, InMemoryRunStore, RunSpec, RunStore};
use starter_flow_nodes::ai_agent::{AgentInputKind, AiAgent, StaticAiRunnerRegistry};
use starter_flow_nodes::log::{self, Log};
use starter_flow_nodes::tool_registry::{StaticToolRegistry, ToolRegistry};
use starter_flow_nodes::trigger_explicit::{
    StaticTriggerChannelRegistry, TriggerChannelRegistry, TriggerExplicit, CHANNEL_ID_SLOT,
    PAYLOAD_SLOT,
};
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowEvent, FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};

const CHANNEL_ID: &str = "examples.notes.codeless-demo";
const PROVIDER_ID: &str = "test.recording";
const TRIGGER_NODE: &str = "smoke.flow.codeless.trigger";
const AGENT_NODE: &str = "smoke.flow.codeless.agent";
const LOG_NODE: &str = "smoke.flow.codeless.log";
const FLOW_ID: &str = "smoke.flow.codeless-shape-on-one-engine";

const SCRIPTED_REPLY: &str = "summary: hello demo";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codeless_shape_trigger_ai_agent_log_chains_through_engine() {
    // --- AI runner registry: RecordingAiRunner under PROVIDER_ID. ---
    let runner = RecordingAiRunner::new(vec![ScriptTurn::text(SCRIPTED_REPLY)]);
    let mut ai_runners = StaticAiRunnerRegistry::new();
    ai_runners.register(KindId::new(PROVIDER_ID).unwrap(), runner.clone());
    let ai_runners_arc: Arc<dyn AiRunnerRegistry> = Arc::new(ai_runners);

    // --- Trigger channel registry + sender bound to CHANNEL_ID. ---
    let mut tcr = StaticTriggerChannelRegistry::new();
    let sender = tcr.bind(KindId::new(CHANNEL_ID).unwrap(), 4);
    let tcr_arc: Arc<dyn TriggerChannelRegistry> = Arc::new(tcr);

    // --- Empty tool registry (Cli path doesn't dispatch through it). ---
    let tools: Arc<dyn ToolRegistry> = Arc::new(StaticToolRegistry::new());

    // --- Node bodies. ---
    let trigger_body: Arc<dyn NodeBehavior> = Arc::new(TriggerExplicit::new(tcr_arc));
    let agent_body: Arc<dyn NodeBehavior> = Arc::new(
        AiAgent::new(tools, ai_runners_arc)
            .with_provider_id(KindId::new(PROVIDER_ID).unwrap())
            .with_input_kind(AgentInputKind::Cli),
    );
    let log_body: Arc<dyn NodeBehavior> = Arc::new(Log::new());

    // --- Topology: trigger.payload → agent.input ; agent.output → log.value ---
    let trigger_node = NodeId::new(TRIGGER_NODE).unwrap();
    let agent_node = NodeId::new(AGENT_NODE).unwrap();
    let log_node = NodeId::new(LOG_NODE).unwrap();

    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(trigger_node.clone(), trigger_body);
    behaviors.insert(agent_node.clone(), agent_body);
    behaviors.insert(log_node.clone(), log_body);

    let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
    links.insert(
        SlotRef::new(trigger_node.clone(), PAYLOAD_SLOT),
        vec![SlotRef::new(
            agent_node.clone(),
            starter_flow_nodes::ai_agent::INPUT_SLOT,
        )],
    );
    links.insert(
        SlotRef::new(
            agent_node.clone(),
            starter_flow_nodes::ai_agent::OUTPUT_SLOT,
        ),
        vec![SlotRef::new(log_node.clone(), log::VALUE_SLOT)],
    );

    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    triggers.insert(
        trigger_node.clone(),
        std::iter::once(CHANNEL_ID_SLOT.to_owned()).collect(),
    );
    triggers.insert(
        agent_node.clone(),
        std::iter::once(starter_flow_nodes::ai_agent::INPUT_SLOT.to_owned()).collect(),
    );
    triggers.insert(
        log_node.clone(),
        std::iter::once(log::VALUE_SLOT.to_owned()).collect(),
    );

    let topology = Arc::new(FlowTopology {
        links,
        triggers,
        reads: BTreeMap::new(),
        behaviors,
    });

    // --- Runner + stores. ---
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let flow_runner = FlowRunner::new(graph_store.clone(), run_store);

    // --- Fire BEFORE start: the mpsc buffer absorbs the race. ---
    sender
        .fire(serde_json::json!({"prompt": "summarise the demo note"}))
        .await
        .expect("fire ok");

    // --- Start the run with the trigger's channel_id slot as seed. ---
    let seeds = vec![(
        SlotRef::new(trigger_node.clone(), CHANNEL_ID_SLOT),
        SlotValue::String(CHANNEL_ID.to_owned()),
    )];
    let terminal_slots = vec![SlotRef::new(log_node.clone(), log::EMITTED_SLOT)];
    let spec = RunSpec::new(
        FlowId::new(FLOW_ID).unwrap(),
        FlowRevisionId::new(),
        topology,
        seeds,
        terminal_slots,
    );
    let mut handle = flow_runner
        .start(spec, SlotMap::new())
        .await
        .expect("start ok");

    // --- Drain events to RunCompleted. ---
    let mut completed_output: Option<SlotMap> = None;
    tokio::time::timeout(Duration::from_secs(5), async {
        use tokio::sync::broadcast::error::RecvError;
        loop {
            match handle.initial_rx.recv().await {
                Ok(FlowEvent::RunCompleted { output, .. }) => {
                    completed_output = Some(output);
                    return;
                }
                Ok(FlowEvent::RunFailed { error, .. }) => {
                    panic!("run failed: {error}");
                }
                Ok(FlowEvent::RunCancelled { .. }) => {
                    panic!("run cancelled unexpectedly");
                }
                Ok(_) => {}
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return,
            }
        }
    })
    .await
    .expect("run completed within 5s");

    // --- Assertions. ---

    // 1. RecordingAiRunner saw exactly one call — proves the
    //    trigger payload reached the ai-agent through the propagator.
    let calls = runner.calls();
    assert_eq!(
        calls.len(),
        1,
        "ai-agent invoked exactly once; got {calls:?}"
    );

    // 2. RunCompleted's terminal output map carries the log node's
    //    passthrough `emitted` slot with the ai-agent's scripted
    //    reply — proves agent.output → log.value plumbing AND that
    //    the log body actually ran (the passthrough only writes if
    //    invoke completed cleanly).
    let out = completed_output.expect("run completed with output");
    let emitted_key = format!("{LOG_NODE}.{}", log::EMITTED_SLOT);
    let emitted = out
        .get(&emitted_key)
        .unwrap_or_else(|| panic!("terminal output missing `{emitted_key}`; got {out:?}"));
    match emitted {
        SlotValue::String(s) => assert_eq!(s, SCRIPTED_REPLY),
        other => panic!("expected String on log.emitted; got {other:?}"),
    }
}
