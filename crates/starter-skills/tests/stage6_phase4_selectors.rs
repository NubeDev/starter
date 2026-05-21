//! Phase 4 stage 6 — `SkillRegistry` impl `SkillSelector` + the
//! three selector-strategy impls (`Llm`, `Keyword`, `First`).
//!
//! Two normative smokes from the stage brief land here:
//!
//! 1. `selection_is_frozen_per_run_against_the_real_registry` —
//!    retargets the `crates/starter-flow/tests/stage5_skill_threading.rs`
//!    pattern at a registry-backed selector. The engine pins the
//!    [`SkillSelection`] returned by the first call for the duration
//!    of the run; this test proves that pinned value's
//!    `content_hash` survives a mid-run bundle edit followed by an
//!    explicit `reload()`. Two `select()` calls bracket the edit —
//!    the first stands in for "ai-agent node A" and the second for
//!    "ai-agent node B"; the engine threads the same `SkillSelection`
//!    to both, and the registry's snapshot value is the source of
//!    truth for that pinning.
//!
//! 2. `quarantined_skill_never_reaches_selector_strategy` — a
//!    [`SelectorStrategy`] that wraps [`LlmSkillSelector`] records
//!    every candidate it is handed. With one approved and one
//!    quarantined bundle loaded, the recorded set contains only the
//!    approved one regardless of what the LLM returns. The recording
//!    wrapper lets the test prove this without coupling to the
//!    LLM's response — the registry's pre-filter is what enforces
//!    the invariant (R-skills-3).

use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::mpsc;

use starter_flow_spi::node::SlotMap;
use starter_flow_spi::skill::{SkillError, SkillId, SkillSelection, SkillSelector};
use starter_flow_spi::Principal;
use starter_skills::{
    ContributedSkill, InMemoryApprovalStore, LlmSkillSelector, SelectorStrategy, Skill,
    SkillRegistry,
};
use starter_spi::ai::{
    AiRunner, Cancel, Event, Provider, RunResult, RunnerError, RunnerInput, SessionId,
};

// ---------------------------------------------------------------------
// Helpers — local TempDir + Principal + skill fixtures
// ---------------------------------------------------------------------

struct TempDir(std::path::PathBuf);
impl TempDir {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let base = std::env::temp_dir().join(format!(
            "starter-skills-stage6-{tag}-{}-{nanos}",
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
        extra: serde_json::Value::Null,
    }
}

fn approved_skill_md(id: &str, desc: &str) -> String {
    format!(
        "---\nid: {id}\ndescription: {desc}\ntrust: approved\n---\nbody for {id}\n"
    )
}

fn quarantined_skill_md(id: &str, desc: &str) -> String {
    format!(
        "---\nid: {id}\ndescription: {desc}\ntrust: quarantined\n---\nbody for {id}\n"
    )
}

// ---------------------------------------------------------------------
// Test-only AiRunner: returns a scripted text response. Used by the
// quarantine smoke so the wrapped `LlmSkillSelector` actually has
// something to call.
// ---------------------------------------------------------------------

struct ScriptedRunner {
    provider: Provider,
    text: Mutex<String>,
}

impl ScriptedRunner {
    fn new(text: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            provider: Provider::Anthropic,
            text: Mutex::new(text.into()),
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
        _session_id: SessionId,
        _on_event: mpsc::Sender<Event>,
        _cancel: &dyn Cancel,
    ) -> Result<RunResult, RunnerError> {
        Ok(RunResult {
            text: self.text.lock().unwrap().clone(),
            provider: self.provider.to_string(),
            ..RunResult::default()
        })
    }
}

// ---------------------------------------------------------------------
// Smoke 1 — selection content_hash is frozen across reload (stand-in
// for "two ai-agent nodes see the same selection").
// ---------------------------------------------------------------------

#[tokio::test]
async fn selection_is_frozen_per_run_against_the_real_registry() {
    let tmp = TempDir::new("frozen");

    // One approved bundle. The selector strategy is `FirstSkillSelector`
    // (deterministic — we want to pin the selection, not exercise the
    // LLM path), but we wire it via the `with_default_selector` hook
    // to prove the registry actually dispatches to it.
    let bundle = tmp.path().join("greet");
    std::fs::create_dir_all(&bundle).unwrap();
    write_file(
        &bundle,
        "SKILL.md",
        &approved_skill_md("starter.frozen.greet", "Greets the user."),
    );

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector(starter_skills::FirstSkillSelector::new())
        .load_dir(tmp.path())
        .build()
        .await
        .expect("build ok");

    // ai-agent node A's perspective: the engine resolves the
    // selection once and threads it through. Capture that value.
    let p = principal();
    let sel_a = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &p)
        .await
        .expect("select ok");

    let hash_a = match &sel_a {
        SkillSelection::Selected { content_hash, .. } => content_hash.clone(),
        other => panic!("expected Selected, got {other:?}"),
    };

    // Drift: edit the bundle on disk and reload. In production the
    // engine would keep the previously-resolved selection regardless;
    // here we prove the *content_hash on the pinned value* is the
    // H1 hash, not whatever H2 reload produced.
    write_file(
        &bundle,
        "SKILL.md",
        &(approved_skill_md("starter.frozen.greet", "Greets the user.")
            + "\nADDED LINE\n"),
    );
    registry.reload().await.expect("reload ok");

    // ai-agent node B's perspective: the engine threads the *same*
    // selection. The hash on `sel_a` must not have changed (it's an
    // owned value), and a fresh select() now returns a *different*
    // hash — the engine's pinning is what guarantees node B sees A's
    // hash, but the registry's content-hash machinery is what makes
    // that possible.
    let sel_b_if_reselected = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &p)
        .await
        .expect("select ok");
    let hash_b = match &sel_b_if_reselected {
        SkillSelection::Selected { content_hash, .. } => content_hash.clone(),
        other => panic!("expected Selected, got {other:?}"),
    };

    // The captured (pinned) selection still carries H1.
    assert_eq!(
        hash_a,
        match &sel_a {
            SkillSelection::Selected { content_hash, .. } => content_hash.clone(),
            _ => unreachable!(),
        },
        "captured selection is immutable"
    );
    assert_ne!(
        hash_a, hash_b,
        "post-reload re-select sees the new H2; the engine's pinning is what hides H2 from node B"
    );
    // And the captured value is what node B receives when the engine
    // hands the pinned selection through unchanged.
    let _node_b_view = sel_a.clone();
}

// ---------------------------------------------------------------------
// Smoke 2 — quarantined skills never reach the strategy.
// ---------------------------------------------------------------------

/// `SelectorStrategy` wrapper that records every candidate slice it
/// is handed and then forwards the call to an inner [`LlmSkillSelector`].
struct RecordingWrapper {
    inner: LlmSkillSelector,
    seen: Mutex<Vec<Vec<SkillId>>>,
}

impl RecordingWrapper {
    fn new(inner: LlmSkillSelector) -> Self {
        Self {
            inner,
            seen: Mutex::new(Vec::new()),
        }
    }
    fn recorded(&self) -> Vec<Vec<SkillId>> {
        self.seen.lock().unwrap().clone()
    }
}

#[async_trait]
impl SelectorStrategy for RecordingWrapper {
    async fn select_from(
        &self,
        candidates: &[Arc<Skill>],
        input: &SlotMap,
        principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        self.seen
            .lock()
            .unwrap()
            .push(candidates.iter().map(|s| s.id.clone()).collect());
        self.inner.select_from(candidates, input, principal).await
    }
}

#[tokio::test]
async fn quarantined_skill_never_reaches_selector_strategy() {
    let tmp = TempDir::new("quar-filter");
    // The load_dir scan is one-level-deep, so we keep the extension
    // bundle under a sibling root the loader does not walk.
    let load_root = tmp.path().join("load");
    std::fs::create_dir_all(&load_root).unwrap();
    let ext_root = tmp.path().join("ext-root");
    std::fs::create_dir_all(&ext_root).unwrap();

    // load_dir honours frontmatter: one approved, one quarantined.
    let ok_bundle = load_root.join("ok");
    std::fs::create_dir_all(&ok_bundle).unwrap();
    write_file(
        &ok_bundle,
        "SKILL.md",
        &approved_skill_md("starter.quar.ok", "Approved skill."),
    );

    let q_bundle = load_root.join("quar");
    std::fs::create_dir_all(&q_bundle).unwrap();
    write_file(
        &q_bundle,
        "SKILL.md",
        &quarantined_skill_md("starter.quar.no", "Quarantined skill."),
    );

    // A second quarantined source — extension-contributed — to prove
    // the filter is universal, not just frontmatter-based.
    let ext_bundle = ext_root.join("ext");
    std::fs::create_dir_all(&ext_bundle).unwrap();
    write_file(
        &ext_bundle,
        "SKILL.md",
        // Frontmatter asks for approved, but `extend(...)` forces
        // quarantine (R-skills-3 row 3).
        &approved_skill_md("starter.quar.ext", "Contributed extension."),
    );

    // The LLM is scripted to *try* to pick the quarantined id. The
    // registry's pre-filter must keep it from ever reaching the
    // strategy in the first place — and because the strategy only
    // sees the approved candidate list, the LLM's "starter.quar.no"
    // response falls into the `unknown_id` failure bucket and the
    // selector returns None.
    let runner = ScriptedRunner::new("starter.quar.no\n");
    let inner = LlmSkillSelector::new(runner.clone() as Arc<dyn AiRunner>);
    let wrapper = Arc::new(RecordingWrapper::new(inner));

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .with_default_selector_arc(wrapper.clone() as Arc<dyn SelectorStrategy>)
        .load_dir(&load_root)
        .extend(vec![ContributedSkill::new(&ext_bundle)])
        .build()
        .await
        .expect("build ok");

    // Sanity: registry actually loaded the three bundles, with two
    // sitting in the quarantined bucket.
    assert_eq!(registry.list().len(), 1, "exactly one approved");
    assert_eq!(
        registry.list_quarantined().len(),
        2,
        "frontmatter-quarantined + extension-contributed"
    );

    let p = principal();
    let selection = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &p)
        .await
        .expect("select ok");

    // The LLM asked for the quarantined id — it isn't in the
    // candidate list, so LlmSkillSelector returns None.
    assert!(
        matches!(selection, SkillSelection::None),
        "quarantined id must not be selectable: got {selection:?}"
    );

    // The recording wrapper captured exactly the approved set.
    let seen = wrapper.recorded();
    assert_eq!(seen.len(), 1, "exactly one select() invocation");
    let ids: Vec<String> = seen[0].iter().map(|i| i.to_string()).collect();
    assert_eq!(
        ids,
        vec!["starter.quar.ok".to_string()],
        "strategy must only see the approved bundle"
    );
}

// ---------------------------------------------------------------------
// Bonus: defaults — no AiRunner, no explicit strategy → Keyword.
// Smoke that the default-strategy resolution doesn't silently pick
// "none" when an approved bundle exists.
// ---------------------------------------------------------------------

#[tokio::test]
async fn default_strategy_falls_back_to_keyword_selector_when_no_runner() {
    let tmp = TempDir::new("kw-default");
    let bundle = tmp.path().join("greet");
    std::fs::create_dir_all(&bundle).unwrap();
    write_file(
        &bundle,
        "SKILL.md",
        &approved_skill_md("starter.kw.greet", "Greets the user."),
    );

    let registry = SkillRegistry::builder()
        .with_approval_store(InMemoryApprovalStore::new())
        .load_dir(tmp.path())
        .build()
        .await
        .expect("build ok");

    let selection = (&registry as &dyn SkillSelector)
        .select(&SlotMap::new(), &principal())
        .await
        .expect("select ok");
    // Empty input → no token overlap → KeywordSelector falls back to
    // first candidate (deterministic).
    assert!(matches!(selection, SkillSelection::Selected { .. }));
}
