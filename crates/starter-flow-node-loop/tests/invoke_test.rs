//! Drives a fresh `AiAgentNode` end-to-end against a [`MockAiRunner`]
//! through the [`NodeBehavior::invoke`] surface and asserts the
//! model's reply lands on the `out` slot.
//!
//! The starter-flow engine drops in transparently around this seam —
//! it constructs the same `NodeCtx` shape and reads the returned
//! `SlotMap` the same way — so a direct call here covers the same
//! contract without dragging a graph store and a propagator into
//! every test.

use std::sync::Arc;

use starter_ai_agent::testing::MockAiRunner;
use starter_flow_node_loop::{AiAgentNode, KIND_ID, OUT_SLOT_REPLY};
use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;
use starter_spi::ai::RunResult;

struct NoopCancel;
impl Cancel for NoopCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

#[tokio::test]
async fn ai_agent_node_reply_lands_in_out_slot() {
    let runner = Arc::new(MockAiRunner::new(vec![RunResult {
        text: "hello from the model".to_owned(),
        ..Default::default()
    }]));
    let node = AiAgentNode::new(runner, Vec::new());
    assert_eq!(node.kind_id().as_str(), KIND_ID);

    let node_id = NodeId::new("com.example.agent").expect("node id");
    let cancel = NoopCancel;
    let skill = SkillSelection::None;
    let ctx = NodeCtx::new(RunId::new(), &node_id, &cancel, &skill);

    let mut input = SlotMap::new();
    input.insert(
        "prompt".to_owned(),
        SlotValue::String("hi there".to_owned()),
    );
    let out = node.invoke(ctx, input).await.expect("invoke ok");
    match out.get(OUT_SLOT_REPLY).expect("out slot present") {
        SlotValue::String(s) => assert_eq!(s, "hello from the model"),
        other => panic!("expected string on `out`, got {other:?}"),
    }
}
