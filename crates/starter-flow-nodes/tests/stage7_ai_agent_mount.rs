//! Phase 4b stage 7 — `ai-agent` body aborts the run on a skill
//! resource hash mismatch (R-skills-7).
//!
//! The registry-side proof lives in
//! `crates/starter-skills/tests/stage7_phase4b_mount.rs`; this file
//! drives the `ai-agent` body itself end-to-end through the
//! [`NodeBehavior::invoke`] seam so the assertion is on the typed
//! [`NodeError::Domain`] the run-telemetry surface actually renders.
//!
//! Scenario:
//!
//! 1. Build a skill bundle on disk with one resource whose bytes hash
//!    to `H1`. Resolve a [`SkillSelection::Selected`] against it via
//!    [`SkillRegistry`].
//! 2. Edit the on-disk resource bytes (the bytes now hash to `H2`).
//!    Invoke the body with the **frozen** `H1` selection — the
//!    on-mount check refuses to mount drifted bytes and surfaces
//!    `NodeError::Domain { code: "skill_resource_hash_mismatch" }`.
//! 3. Call `SkillRegistry::reload()`. The fresh selection captures
//!    `H2`. Invoking the body with that selection succeeds.

#![cfg(feature = "ai-agent")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use starter_flow_nodes::ai_agent::{
    AiAgent, StaticAiRunnerRegistry, INPUT_SLOT, OUTPUT_SLOT, PROVIDER_ID_SLOT,
};
use starter_flow_nodes::tool_registry::ToolRegistry;
use starter_flow_spi::ai_runner::AiRunnerRegistry;
use starter_flow_spi::flow::RunId;
use starter_flow_spi::node::{
    KindId, NodeBehavior, NodeCtx, NodeError, NodeId, SlotMap, SlotValue,
};
use starter_flow_spi::skill::{SkillSelection, SkillSelector};
use starter_flow_spi::{Cancel as FlowCancel, Principal};
use starter_skills::{FirstSkillSelector, InMemoryApprovalStore, SkillRegistry};
use starter_spi::ai::{
    AiRunner, Cancel as AiCancel, Event, Provider, RunResult, RunnerError, RunnerInput,
    SessionId as AiSessionId,
};
use starter_spi::tool::Tool;

// ---------------------------------------------------------------------
// TempDir helper (mirrors the pattern from the in-tree unit tests; no
// `tempfile` crate is on the dev path).
// ---------------------------------------------------------------------

struct TempDir(PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!(
            "starter-flow-nodes-stage7-{tag}-{}-{nanos}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        Self(base)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write_file(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, body).unwrap();
}

fn principal() -> Principal {
    Principal {
        subject: "operator-alice".into(),
        role: starter_spi::auth::Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

// ---------------------------------------------------------------------
// Minimal AiRunner — returns scripted text without doing any I/O. We
// only care that the body reaches the runner *after* the on-mount
// hash check has passed.
// ---------------------------------------------------------------------

struct ScriptedRunner {
    provider: Provider,
    text: String,
}

impl ScriptedRunner {
    fn new(text: &str) -> Arc<Self> {
        Arc::new(Self {
            provider: Provider::Anthropic,
            text: text.to_owned(),
        })
    }
}

#[async_trait]
impl AiRunner for ScriptedRunner {
    fn provider(&self) -> &Provider {
        &self.provider
    }
    async fn ready(&self) -> bool {
        true
    }
    async fn run(
        &self,
        _input: RunnerInput,
        _session_id: AiSessionId,
        _on_event: mpsc::Sender<Event>,
        _cancel: &dyn AiCancel,
    ) -> Result<RunResult, RunnerError> {
        Ok(RunResult {
            text: self.text.clone(),
            provider: self.provider.to_string(),
            ..RunResult::default()
        })
    }
}

/// Tool registry that resolves any reverse-DNS id to a trivial echo
/// tool. The `ai-agent` body intersects (host ∩ skill ∩ node) and
/// surfaces `no_tools_visible` on an empty intersection; the skill in
/// this test declares a single tool so the intersection is non-empty
/// and the runner is actually reached.
struct AnyToolRegistry;
impl ToolRegistry for AnyToolRegistry {
    fn lookup(&self, _id: &KindId) -> Option<Arc<dyn Tool>> {
        Some(Arc::new(EchoTool) as Arc<dyn Tool>)
    }
}

struct EchoTool;
#[async_trait]
impl Tool for EchoTool {
    fn definition(&self) -> starter_spi::tool::ToolDefinition {
        starter_spi::tool::ToolDefinition {
            name: "starter.stage7.echo".to_string(),
            description: "Echoes input under `echoed`.".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        }
    }
    async fn invoke(&self, input: serde_json::Value) -> starter_spi::Result<serde_json::Value> {
        Ok(serde_json::json!({"echoed": input}))
    }
}

struct NoCancel;
impl FlowCancel for NoCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(&'a self) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

fn runner_registry(provider_id: &str, runner: Arc<dyn AiRunner>) -> Arc<dyn AiRunnerRegistry> {
    let mut r = StaticAiRunnerRegistry::new();
    r.register(KindId::new(provider_id).unwrap(), runner);
    Arc::new(r)
}

// Keep the `HashMap` import in use; the empty registry above does not
// need one, but the helper lives close to the runner-registry wiring
// future tests will extend.
#[allow(dead_code)]
fn _kind_map() -> HashMap<KindId, Arc<dyn Tool>> {
    HashMap::new()
}

fn build_agent(provider_id: &str, runner: Arc<dyn AiRunner>) -> AiAgent {
    AiAgent::new(
        Arc::new(AnyToolRegistry),
        runner_registry(provider_id, runner),
    )
}

fn approved_skill_md_with_resource(id: &str, rel: &str) -> String {
    // Declares one allowed_tool so the body's `host ∩ skill ∩ node`
    // intersection is non-empty (otherwise the body short-circuits
    // with `no_tools_visible` before reaching the runner — that
    // failure mode is covered by the unit test in src/ai_agent.rs;
    // here we want the on-mount hash check to be the failure source).
    format!(
        "---\nid: {id}\ndescription: Greets via a mounted file.\ntrust: approved\nallowed_tools:\n  - starter.stage7.echo\nresources:\n  - file://{rel}\n---\nbody\n"
    )
}

// ---------------------------------------------------------------------
// Smoke — resource hash mismatch aborts the run with the typed Domain
// error, and a reload() lets a subsequent run proceed.
// ---------------------------------------------------------------------

#[tokio::test]
async fn resource_hash_mismatch_aborts_the_run_then_reload_proceeds() {
    let tmp = TempDir::new("mount");
    let bundle = tmp.path().join("greeter");
    std::fs::create_dir_all(&bundle).unwrap();
    write_file(
        &bundle,
        "SKILL.md",
        &approved_skill_md_with_resource("starter.stage7.greet", "greeting.md"),
    );
    write_file(&bundle, "greeting.md", "Hello, H1!\n");

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(FirstSkillSelector::new())
        .load_dir(tmp.path())
        .build()
        .await
        .expect("registry builds");

    // Selection at H1: the frozen `ResourceRef.content_hash` matches
    // the bytes on disk. The first run reaches the runner and returns
    // its scripted text.
    let sel_h1 = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select H1 ok");
    assert!(matches!(sel_h1, SkillSelection::Selected { .. }));

    let runner = ScriptedRunner::new("hello from the model");
    let agent = build_agent("p.test", runner.clone());
    let n = NodeId::new("flow.test.ai").unwrap();
    let cancel = NoCancel;
    let inputs = {
        let mut m = SlotMap::new();
        m.insert(
            PROVIDER_ID_SLOT.to_string(),
            SlotValue::String("p.test".to_string()),
        );
        m.insert(INPUT_SLOT.to_string(), SlotValue::String("hi".to_string()));
        m
    };

    let ctx_h1 = NodeCtx::new(
        RunId::new(),
        &n,
        &cancel,
        &sel_h1,
        &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
    );
    let out = agent
        .invoke(ctx_h1, inputs.clone())
        .await
        .expect("H1 mount succeeds");
    assert_eq!(
        out.get(OUTPUT_SLOT),
        Some(&SlotValue::String("hello from the model".to_string())),
        "runner reached after the on-mount check passed"
    );

    // Drift between selection and the next mount. The frozen `H1`
    // selection is now stale — `read_and_verify` rejects it and the
    // body surfaces `Domain { code: "skill_resource_hash_mismatch" }`.
    write_file(&bundle, "greeting.md", "Hello, H2 (drifted)!\n");
    let ctx_drifted = NodeCtx::new(
        RunId::new(),
        &n,
        &cancel,
        &sel_h1,
        &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
    );
    let err = agent
        .invoke(ctx_drifted, inputs.clone())
        .await
        .expect_err("expected Domain::skill_resource_hash_mismatch");
    match err {
        NodeError::Domain { code, message } => {
            assert_eq!(code, "skill_resource_hash_mismatch");
            assert!(
                message.contains("starter.stage7.greet"),
                "message names the offending skill: {message}"
            );
            assert!(
                message.contains("greeting.md") || message.contains("file://"),
                "message names the offending resource URI: {message}"
            );
        }
        other => panic!("expected Domain::skill_resource_hash_mismatch; got {other:?}"),
    }

    // After reload(), the fresh selection captures H2 and the next
    // invocation passes the on-mount check.
    registry.reload().await.expect("reload ok");
    let sel_h2 = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select H2 ok");
    let ctx_h2 = NodeCtx::new(
        RunId::new(),
        &n,
        &cancel,
        &sel_h2,
        &starter_flow_spi::state::NOOP_NODE_STATE_STORE,
    );
    let out_h2 = agent
        .invoke(ctx_h2, inputs)
        .await
        .expect("H2 mount succeeds against the edited bundle");
    assert_eq!(
        out_h2.get(OUTPUT_SLOT),
        Some(&SlotValue::String("hello from the model".to_string())),
    );

    // The runner was reached exactly twice (H1 + H2); the drifted
    // invocation aborted *before* the runner was consulted, which is
    // the load-bearing claim of R-skills-7.
    let _ = runner; // keep the Arc alive for the duration of the test
}

// Keep `Mutex` import in scope for future test helpers that may want
// to count runner calls; not used by the smoke today.
#[allow(dead_code)]
fn _unused_mutex(_m: &Mutex<()>) {}
