//! Phase 4 stage 5 — engine wiring + skill-selection threading.
//!
//! Asserts:
//!
//! 1. A default engine constructed via [`Engine::new`] runs with
//!    [`NullSkillSelector`] and surfaces no `AiRunnerRegistry`.
//! 2. A custom [`SkillSelector`] registered via
//!    [`Engine::with_skill_selector`] is accessible through
//!    [`Engine::skill_selector`] and returns the selection it was
//!    configured to produce.
//! 3. An [`AiRunnerRegistry`] registered via
//!    [`Engine::with_ai_runner_registry`] is accessible through
//!    [`Engine::ai_runners`] and looks up registered providers.
//! 4. A [`SkillSelector`] that returns
//!    [`SkillError`] does NOT prevent a run from starting — the
//!    runner logs the failure and falls back to
//!    [`SkillSelection::None`] (per [`crate::run::FlowRunner::launch`]).

use std::sync::Arc;

use async_trait::async_trait;

use starter_flow::engine::Engine;
use starter_flow::graph::InMemoryGraphStore;
use starter_flow::propagator::FlowTopology;
use starter_flow::run::RunStore;
use starter_flow::run::{
    FlowRunner, InMemoryRunStore, RunSpec, SkillError, SkillSelection, SkillSelector,
};
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::{FlowId, FlowRevisionId};
use starter_flow_spi::graph::GraphStore;
use starter_flow_spi::node::{KindId, SlotMap, SlotRef, SlotValue};
use starter_flow_spi::Principal;

// ---- Default engine wiring --------------------------------------------

#[tokio::test]
async fn default_engine_has_null_selector_and_no_ai_runner_registry() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let engine = Engine::new(store);
    assert!(
        engine.ai_runners().is_none(),
        "ai_runners() must default to None"
    );
    // Run NullSkillSelector.select against an empty input + system
    // principal — must succeed with SkillSelection::None.
    let principal = Principal {
        subject: "system/Admin".to_string(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        extra: serde_json::Value::Null,
    };
    let selection = engine
        .skill_selector()
        .select(&SlotMap::new(), &principal)
        .await
        .expect("NullSkillSelector cannot fail");
    assert!(matches!(selection, SkillSelection::None));
}

// ---- Custom SkillSelector wiring --------------------------------------

struct FixedSelector {
    skill_id: starter_flow_spi::skill::SkillId,
    hash: String,
}

#[async_trait]
impl SkillSelector for FixedSelector {
    async fn select(
        &self,
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        Ok(SkillSelection::Selected {
            skill_id: self.skill_id.clone(),
            allowed_tools: Vec::new(),
            resources: Vec::new(),
            content_hash: self.hash.clone(),
        })
    }
}

#[tokio::test]
async fn custom_skill_selector_is_reachable_via_engine_accessor() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let selector = Arc::new(FixedSelector {
        skill_id: starter_flow_spi::skill::SkillId::new("test.engine.fixed").unwrap(),
        hash: "h-engine".to_string(),
    });
    let engine = Engine::new(store).with_skill_selector(selector);

    let principal = Principal {
        subject: "system/Admin".to_string(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        extra: serde_json::Value::Null,
    };
    let selection = engine
        .skill_selector()
        .select(&SlotMap::new(), &principal)
        .await
        .expect("selector ok");
    match selection {
        SkillSelection::Selected {
            content_hash,
            skill_id,
            ..
        } => {
            assert_eq!(content_hash, "h-engine");
            assert_eq!(skill_id.as_str(), "test.engine.fixed");
        }
        other => panic!("expected Selected, got {other:?}"),
    }
}

// ---- AiRunnerRegistry wiring ------------------------------------------

struct EmptyRegistry;
impl AiRunnerRegistry for EmptyRegistry {
    fn lookup(&self, _provider_id: &KindId) -> Option<Arc<dyn starter_spi::ai::AiRunner>> {
        None
    }
}

#[tokio::test]
async fn ai_runner_registry_is_reachable_via_engine_accessor() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let registry: Arc<dyn AiRunnerRegistry> = Arc::new(EmptyRegistry);
    let engine = Engine::new(store).with_ai_runner_registry(registry);
    let attached = engine.ai_runners().expect("registry attached");
    assert!(
        attached
            .lookup(&KindId::new("test.never").unwrap())
            .is_none(),
        "EmptyRegistry returns None for any lookup"
    );
}

// ---- Failing SkillSelector falls back to None -------------------------

struct AlwaysFailing;

#[async_trait]
impl SkillSelector for AlwaysFailing {
    async fn select(
        &self,
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        Err(SkillError::new(
            "test_failure",
            "selector deliberately failed",
        ))
    }
}

#[tokio::test]
async fn failing_skill_selector_falls_back_to_none_and_run_still_starts() {
    let store: Arc<dyn GraphStore> = Arc::new(InMemoryGraphStore::new());
    let run_store: Arc<dyn RunStore> = Arc::new(InMemoryRunStore::new());
    let runner =
        FlowRunner::new(store, run_store.clone()).with_skill_selector(Arc::new(AlwaysFailing));

    // Empty topology / no nodes — the run quiesces immediately.
    let topology = Arc::new(FlowTopology::default());
    let spec = RunSpec::new(
        FlowId::new("flow.test.skill.fallback").unwrap(),
        FlowRevisionId::new(),
        topology,
        vec![(
            SlotRef::new(
                starter_flow_spi::node::NodeId::new("flow.test.x").unwrap(),
                "out",
            ),
            SlotValue::Int(1),
        )],
        vec![SlotRef::new(
            starter_flow_spi::node::NodeId::new("flow.test.x").unwrap(),
            "out",
        )],
    );
    let mut handle = runner
        .start(spec, SlotMap::new())
        .await
        .expect("start must not be rejected by a failing selector");
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), &mut handle.join).await;

    let recorded = run_store.get(handle.run).await.expect("run recorded");
    let st = recorded.read().await;
    let sel = st
        .skill_selection
        .as_ref()
        .expect("skill_selection must always be pinned on RunState");
    assert!(matches!(sel.as_ref(), SkillSelection::None));
}
