//! Phase 4 stage 8 — SCOPE smoke 2: "Skill quarantine survives
//! bundle update through a flow."
//!
//! Contract from the job WORKFLOW (template.yaml stage 8):
//!
//! > skill selection runs once per flow run; the same SkillSelection
//! > threads through three ai-agent nodes; swapping the underlying
//! > skill bundle mid-flow via the test SkillSelector does not
//! > change the SkillSelection mid-run for the in-flight
//! > invocations.
//!
//! This smoke asserts D-F4.4 quarantine + D-F4.5 tools intersection:
//!
//! 1. A `MutatingSkillSelector` that toggles its internal state
//!    on every call returns the *same* selection content_hash for
//!    every observer of one run (the engine invokes the selector
//!    exactly once per `FlowRunner::start` and freezes the result).
//! 2. A three-node flow (gather → review → summarise), each node
//!    backed by the same `AiAgent` body + `RecordingAiRunner`,
//!    records three calls — all with the same `tools_count`,
//!    proving every node saw the same intersected tools list
//!    derived from the same SkillSelection.
//! 3. A second run, with the selector mutated between runs, sees
//!    the *new* selection (proving the freeze is per-run, not
//!    per-process).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use starter_ai::testing::{RecordingAiRunner, ScriptTurn};
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::{
    FlowRunner, InMemoryRunStore, RunSpec, RunStore, SkillError, SkillSelection, SkillSelector,
};
use starter_flow_nodes::ai_agent::{AiAgent, StaticAiRunnerRegistry, INPUT_SLOT, OUTPUT_SLOT};
use starter_flow_nodes::tool_call::ToolRegistry;
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::skill::SkillId;
use starter_flow_spi::Principal;

const PROVIDER_ID: &str = "test.recording";

/// Selector whose internal state toggles between two "bundle
/// versions". Each call returns the *current* state and then
/// mutates — but the engine guarantees this is called exactly once
/// per `FlowRunner::start`, so the toggle is observable only across
/// runs, never within one.
struct MutatingSkillSelector {
    calls: AtomicU64,
}

#[async_trait]
impl SkillSelector for MutatingSkillSelector {
    async fn select(
        &self,
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        let (hash, tool) = if n == 0 {
            ("v1", "test.tool.read")
        } else {
            ("v2", "test.tool.write")
        };
        Ok(SkillSelection::Selected {
            skill_id: SkillId::new("test.skill.toggle").unwrap(),
            allowed_tools: vec![KindId::new(tool).unwrap()],
            resources: Vec::new(),
            content_hash: hash.to_string(),
        })
    }
}

fn build_three_node_topology(ai_agent: Arc<dyn NodeBehavior>) -> Arc<FlowTopology> {
    let gather = NodeId::new("flow.smoke.gather").unwrap();
    let review = NodeId::new("flow.smoke.review").unwrap();
    let summarise = NodeId::new("flow.smoke.summarise").unwrap();

    let mut triggers: BTreeMap<NodeId, BTreeSet<String>> = BTreeMap::new();
    for n in [&gather, &review, &summarise] {
        triggers.insert(n.clone(), {
            let mut s = BTreeSet::new();
            s.insert(INPUT_SLOT.to_string());
            s
        });
    }

    // Chain: gather.output → review.input ; review.output → summarise.input.
    let mut links: HashMap<SlotRef, Vec<SlotRef>> = HashMap::new();
    links.insert(
        SlotRef::new(gather.clone(), OUTPUT_SLOT),
        vec![SlotRef::new(review.clone(), INPUT_SLOT)],
    );
    links.insert(
        SlotRef::new(review.clone(), OUTPUT_SLOT),
        vec![SlotRef::new(summarise.clone(), INPUT_SLOT)],
    );

    let mut behaviors: BTreeMap<NodeId, Arc<dyn NodeBehavior>> = BTreeMap::new();
    behaviors.insert(gather, ai_agent.clone());
    behaviors.insert(review, ai_agent.clone());
    behaviors.insert(summarise, ai_agent);

    Arc::new(FlowTopology {
        links,
        triggers,
        behaviors,
    })
}

/// A tool with reverse-DNS id `test.tool.read` so the v1
/// selection's `allowed_tools` intersection finds something the
/// host registry can resolve.
struct ReadTool;

#[async_trait]
impl starter_spi::tool::Tool for ReadTool {
    fn definition(&self) -> starter_spi::tool::ToolDefinition {
        starter_spi::tool::ToolDefinition {
            name: "test.tool.read".to_string(),
            description: "Test read tool".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }
    async fn invoke(&self, _input: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
        Ok(serde_json::json!({"ok": true}))
    }
}

struct TestToolRegistry {
    tools: HashMap<KindId, Arc<dyn starter_spi::tool::Tool>>,
}

impl ToolRegistry for TestToolRegistry {
    fn lookup(&self, tool_id: &KindId) -> Option<Arc<dyn starter_spi::tool::Tool>> {
        self.tools.get(tool_id).cloned()
    }
}

fn registry_with_read_tool() -> Arc<dyn ToolRegistry> {
    let mut tools: HashMap<KindId, Arc<dyn starter_spi::tool::Tool>> = HashMap::new();
    tools.insert(
        KindId::new("test.tool.read").unwrap(),
        Arc::new(ReadTool) as Arc<dyn starter_spi::tool::Tool>,
    );
    Arc::new(TestToolRegistry { tools })
}

fn ai_agent(runner: Arc<RecordingAiRunner>) -> Arc<dyn NodeBehavior> {
    let mut ai_runners = StaticAiRunnerRegistry::new();
    ai_runners.register(KindId::new(PROVIDER_ID).unwrap(), runner);
    let ai_runners_arc: Arc<dyn AiRunnerRegistry> = Arc::new(ai_runners);
    let tools = registry_with_read_tool();
    Arc::new(
        AiAgent::new(tools, ai_runners_arc).with_provider_id(KindId::new(PROVIDER_ID).unwrap()),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn skill_selection_threads_unchanged_through_three_ai_agent_nodes() {
    // All three nodes share one RecordingAiRunner; each invocation
    // returns the same "done" text so the chain quiesces after 3 turns.
    let runner = RecordingAiRunner::new(vec![ScriptTurn::text("done")]);
    let body = ai_agent(runner.clone());
    let topology = build_three_node_topology(body);

    let selector = Arc::new(MutatingSkillSelector {
        calls: AtomicU64::new(0),
    });
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let flow_runner = FlowRunner::new(graph_store, run_store).with_skill_selector(selector.clone());

    let spec = RunSpec::new(
        FlowId::new("flow.smoke.skill-quarantine").unwrap(),
        FlowRevisionId::new(),
        topology,
        vec![(
            SlotRef::new(NodeId::new("flow.smoke.gather").unwrap(), INPUT_SLOT),
            SlotValue::String("start".to_string()),
        )],
        vec![SlotRef::new(
            NodeId::new("flow.smoke.summarise").unwrap(),
            OUTPUT_SLOT,
        )],
    );
    let mut handle = flow_runner
        .start(spec, SlotMap::new())
        .await
        .expect("start ok");
    let _ = tokio::time::timeout(Duration::from_secs(5), &mut handle.join)
        .await
        .expect("join within 5s");

    // The selector was called exactly once for this run (D-F4.4
    // outer-run binding); the toggle did NOT fire mid-run.
    assert_eq!(
        selector.calls.load(Ordering::SeqCst),
        1,
        "SkillSelector must be invoked exactly once per FlowRunner::start"
    );

    // All three ai-agent nodes ran; each saw the same advertised
    // tools_count (== 1, the v1 selection's allowed_tools layer
    // intersected against the host registry's `test.tool.read`).
    let calls = runner.calls();
    assert_eq!(
        calls.len(),
        3,
        "three ai-agent nodes must each fire exactly once; got {}",
        calls.len()
    );
    for (i, c) in calls.iter().enumerate() {
        assert_eq!(
            c.tools_count, 1,
            "node {i} must see the same tools intersection (v1 → [test.tool.read]); got {}",
            c.tools_count
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cross_run_selector_mutation_changes_only_the_next_run() {
    // One selector, two sequential runs. The first run sees the v1
    // selection (allowed_tools = ["test.tool.read"]). The second
    // run sees v2 (allowed_tools = ["test.tool.write"]) — the
    // selector's per-call toggle fires once between the two starts.
    let selector = Arc::new(MutatingSkillSelector {
        calls: AtomicU64::new(0),
    });
    let graph_store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let flow_runner =
        FlowRunner::new(graph_store, run_store.clone()).with_skill_selector(selector.clone());

    // Run 1.
    let runner_a = RecordingAiRunner::new(vec![ScriptTurn::text("a")]);
    let topology_a = build_three_node_topology(ai_agent(runner_a.clone()));
    let spec_a = RunSpec::new(
        FlowId::new("flow.smoke.skill-quarantine-run-a").unwrap(),
        FlowRevisionId::new(),
        topology_a,
        vec![(
            SlotRef::new(NodeId::new("flow.smoke.gather").unwrap(), INPUT_SLOT),
            SlotValue::String("first".to_string()),
        )],
        vec![SlotRef::new(
            NodeId::new("flow.smoke.summarise").unwrap(),
            OUTPUT_SLOT,
        )],
    );
    let mut handle_a = flow_runner
        .start(spec_a, SlotMap::new())
        .await
        .expect("start a");
    let _ = tokio::time::timeout(Duration::from_secs(5), &mut handle_a.join)
        .await
        .expect("join a");

    // The selector was invoked once for run 1; the toggle moved it
    // into the v2 state for the next selector call.
    assert_eq!(selector.calls.load(Ordering::SeqCst), 1);
    let recorded_a = run_store.get(handle_a.run).await.expect("a recorded");
    let st_a = recorded_a.read().await;
    let sel_a = st_a.skill_selection.as_ref().expect("a has selection");
    match sel_a.as_ref() {
        SkillSelection::Selected { content_hash, .. } => assert_eq!(content_hash, "v1"),
        other => panic!("run a expected Selected v1, got {other:?}"),
    }
    drop(st_a);

    // Run 2 — fresh graph store to avoid slot collisions on the
    // re-used topology node ids.
    let graph_store_b: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let run_store_b: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let flow_runner_b =
        FlowRunner::new(graph_store_b, run_store_b.clone()).with_skill_selector(selector.clone());

    let runner_b = RecordingAiRunner::new(vec![ScriptTurn::text("b")]);
    let topology_b = build_three_node_topology(ai_agent(runner_b));
    let spec_b = RunSpec::new(
        FlowId::new("flow.smoke.skill-quarantine-run-b").unwrap(),
        FlowRevisionId::new(),
        topology_b,
        vec![(
            SlotRef::new(NodeId::new("flow.smoke.gather").unwrap(), INPUT_SLOT),
            SlotValue::String("second".to_string()),
        )],
        vec![SlotRef::new(
            NodeId::new("flow.smoke.summarise").unwrap(),
            OUTPUT_SLOT,
        )],
    );
    let mut handle_b = flow_runner_b
        .start(spec_b, SlotMap::new())
        .await
        .expect("start b");
    let _ = tokio::time::timeout(Duration::from_secs(5), &mut handle_b.join)
        .await
        .expect("join b");

    assert_eq!(selector.calls.load(Ordering::SeqCst), 2);
    let recorded_b = run_store_b.get(handle_b.run).await.expect("b recorded");
    let st_b = recorded_b.read().await;
    let sel_b = st_b.skill_selection.as_ref().expect("b has selection");
    match sel_b.as_ref() {
        SkillSelection::Selected { content_hash, .. } => assert_eq!(content_hash, "v2"),
        other => panic!("run b expected Selected v2, got {other:?}"),
    }
}
