//! Stage 2 (phase A+B.2) coverage for `starter.flow.counter` — the
//! first consumer of [`starter_flow_spi::state::NodeStateStore`].
//!
//! Scenarios mirror the matrix in the stage spec:
//!
//! - `initial_fire_emits_initial_plus_step`
//! - `second_fire_emits_prior_plus_step`
//! - `settings_change_respects_new_step`
//! - `reset_on_redeploy_true_clears_state`
//! - `reset_on_redeploy_false_preserves_state`

#![cfg(feature = "counter")]

use std::future::Future;
use std::pin::Pin;

use starter_flow_nodes::counter::{Counter, CounterSettings, OUT_SLOT};
use starter_flow_spi::flow::{FlowId, RunId};
use starter_flow_spi::node::{EditKind, NodeBehavior, NodeCtx, NodeId, SlotMap, SlotValue};
use starter_flow_spi::skill::SkillSelection;
use starter_flow_spi::Cancel;

use starter_flow::state::in_memory::InMemoryNodeStateStore;

struct NoCancel;
impl Cancel for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

fn flow() -> FlowId {
    FlowId::new("acme.flows.counter-test").unwrap()
}

fn node() -> NodeId {
    NodeId::new("acme.nodes.counter").unwrap()
}

fn ctx<'a>(
    flow: &'a FlowId,
    node: &'a NodeId,
    cancel: &'a dyn Cancel,
    state: &'a dyn starter_flow_spi::state::NodeStateStore,
) -> NodeCtx<'a> {
    NodeCtx::with_flow(
        flow,
        RunId::new(),
        node,
        cancel,
        SkillSelection::NONE,
        state,
    )
}

fn out_i64(out: &SlotMap) -> i64 {
    match out.get(OUT_SLOT) {
        Some(SlotValue::Int(n)) => *n,
        other => panic!("expected i64 on out slot; got {other:?}"),
    }
}

#[tokio::test]
async fn initial_fire_emits_initial_plus_step() {
    let store = InMemoryNodeStateStore::new();
    let body = Counter::with_settings(CounterSettings {
        step: 1,
        initial: 10,
        reset_on_redeploy: false,
    });
    let (f, n, c) = (flow(), node(), NoCancel);

    let out = body
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .expect("counter invoke");
    assert_eq!(out_i64(&out), 11);
}

#[tokio::test]
async fn second_fire_emits_prior_plus_step() {
    let store = InMemoryNodeStateStore::new();
    let body = Counter::with_settings(CounterSettings::default()); // step=1, initial=0
    let (f, n, c) = (flow(), node(), NoCancel);

    let out1 = body
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(out_i64(&out1), 1);

    let out2 = body
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(out_i64(&out2), 2);
}

#[tokio::test]
async fn settings_change_respects_new_step() {
    let store = InMemoryNodeStateStore::new();
    let (f, n, c) = (flow(), node(), NoCancel);

    let body_step1 = Counter::with_settings(CounterSettings {
        step: 1,
        initial: 0,
        reset_on_redeploy: false,
    });
    let out1 = body_step1
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(out_i64(&out1), 1);

    // Simulate a settings edit: the engine builds a fresh body with
    // the new step value (R5: behaviours are stateless across edits).
    let body_step5 = Counter::with_settings(CounterSettings {
        step: 5,
        initial: 0,
        reset_on_redeploy: false,
    });
    let out2 = body_step5
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(out_i64(&out2), 6, "prior=1 + new_step=5");
}

#[tokio::test]
async fn reset_on_redeploy_true_clears_state() {
    let store = InMemoryNodeStateStore::new();
    let (f, n, c) = (flow(), node(), NoCancel);
    let body = Counter::with_settings(CounterSettings {
        step: 1,
        initial: 0,
        reset_on_redeploy: true,
    });

    let out1 = body
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(out_i64(&out1), 1);

    for edit in [EditKind::Settings, EditKind::Topology, EditKind::Both] {
        // Re-prime state so each edit kind starts from a real value.
        body.invoke(ctx(&f, &n, &c, &store), SlotMap::new())
            .await
            .unwrap();
        body.on_redeploy(ctx(&f, &n, &c, &store), edit)
            .await
            .expect("on_redeploy clears state");
        let out_after = body
            .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
            .await
            .unwrap();
        assert_eq!(
            out_i64(&out_after),
            1,
            "after on_redeploy({edit:?}) the next fire restarts from initial+step",
        );
    }
}

#[tokio::test]
async fn reset_on_redeploy_false_preserves_state() {
    let store = InMemoryNodeStateStore::new();
    let (f, n, c) = (flow(), node(), NoCancel);
    let body = Counter::with_settings(CounterSettings {
        step: 1,
        initial: 0,
        reset_on_redeploy: false,
    });

    body.invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    body.invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap(); // count = 2

    body.on_redeploy(ctx(&f, &n, &c, &store), EditKind::Both)
        .await
        .expect("on_redeploy no-op when reset_on_redeploy=false");

    let out_after = body
        .invoke(ctx(&f, &n, &c, &store), SlotMap::new())
        .await
        .unwrap();
    assert_eq!(
        out_i64(&out_after),
        3,
        "state preserved across redeploy when reset_on_redeploy=false",
    );
}
