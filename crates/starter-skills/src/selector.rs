//! Phase 4 — selector strategies + the [`SkillSelector`] impl on
//! [`crate::SkillRegistry`].
//!
//! ## Roles
//!
//! - [`SelectorStrategy`] — the *narrow* trait the registry dispatches
//!   to. It differs from [`SkillSelector`] in exactly one way: it
//!   receives the **approved-only** candidate list as an argument, so
//!   a strategy cannot accidentally see a quarantined bundle. The
//!   `SkillRegistry` is the only `SkillSelector` impl in this crate;
//!   every other concrete selector is a `SelectorStrategy`.
//! - [`LlmSkillSelector`] — wraps an
//!   [`starter_spi::ai::AiRunner`]. Default model is Haiku; the
//!   builder accepts overrides. **Failure semantics are normative**:
//!   2 second hard timeout, no retries, on timeout / runner error /
//!   upstream error / unknown id return [`SkillSelection::None`] and
//!   emit one WARN tracing event tagged
//!   `target = "skill_selector_failed_total"` with a `reason` field
//!   (the canonical "increment a metric" surface for the v1 crate —
//!   a real Prometheus counter will plug into the same target name
//!   in a later phase). Every outcome (`selected` / `none_chosen` /
//!   the four failure reasons) lands as a span on the
//!   `skill.selector` target.
//! - [`KeywordSkillSelector`] — deterministic, no LLM call. Picks the
//!   alphabetically-first approved skill whose description tokens
//!   intersect the input. Used when no `AiRunner` is configured.
//! - [`FirstSkillSelector`] — test fixture. Always returns the
//!   alphabetically-first approved skill, or `None` if the candidate
//!   list is empty.
//!
//! ## Default strategy resolution (`SkillRegistryBuilder`)
//!
//! Resolved at `build()` time, in priority order:
//!
//! 1. Explicit [`SkillRegistryBuilder::with_default_selector`].
//! 2. [`SkillRegistryBuilder::with_ai_runner`] → [`LlmSkillSelector`]
//!    with default timeout + Haiku model.
//! 3. Neither configured → [`KeywordSkillSelector`].
//!
//! ## Once-per-run threading
//!
//! The engine pins the [`SkillSelection`] this strategy returns for
//! the lifetime of the run (`Engine::with_skill_selector(Arc::new(registry))`).
//! That invariant lives in `starter-flow`; this crate's
//! `selection_is_frozen_across_multiple_select_calls` smoke proves the
//! registry-returned value's `content_hash` is stable even if the
//! underlying bundle is edited and `reload()`-ed between two select
//! calls — i.e. the engine's "select once, thread everywhere"
//! contract survives backing-store mutation.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use starter_flow_spi::node::SlotMap;
use starter_flow_spi::skill::{SkillError, SkillSelection, SkillSelector};
use starter_flow_spi::Principal;
use starter_spi::ai::{AiRunner, Cancel, Event, RestCfg, RunnerInput, SessionId};
use tokio::sync::mpsc;

use crate::registry::{Skill, SkillRegistry};

/// Default per-call timeout for [`LlmSkillSelector`]. Two seconds is
/// the normative budget per the Phase 4 stage brief — anything
/// slower falls back to [`SkillSelection::None`] so a flaky upstream
/// never blocks a flow run.
pub const DEFAULT_LLM_SELECTOR_TIMEOUT: Duration = Duration::from_secs(2);

/// Default model id [`LlmSkillSelector`] passes through to the
/// [`AiRunner`]. Haiku per the Phase 4 stage brief.
///
/// Hosts that need a different default override via
/// [`LlmSkillSelectorBuilder::with_model`]. The runner is free to
/// ignore the hint (S-D3 best-effort).
pub const DEFAULT_LLM_SELECTOR_MODEL: &str = "claude-3-5-haiku-latest";

/// Narrow trait the registry dispatches to.
///
/// Receives the **approved-only** candidate list as an argument —
/// quarantined bundles are filtered out by [`SkillRegistry`] before
/// the strategy is ever called (the
/// `quarantined_skill_never_reaches_selector_strategy` smoke pins
/// this invariant).
#[async_trait]
pub trait SelectorStrategy: Send + Sync + 'static {
    /// Choose at most one skill from `candidates`. Returning
    /// [`SkillSelection::None`] is always valid.
    async fn select_from(
        &self,
        candidates: &[Arc<Skill>],
        input: &SlotMap,
        principal: &Principal,
    ) -> Result<SkillSelection, SkillError>;
}

// ---------------------------------------------------------------------
// LlmSkillSelector
// ---------------------------------------------------------------------

/// LLM-backed selector. Calls the wrapped [`AiRunner`] once against a
/// deterministically-ordered list of `(skill_id, description)` pairs
/// and parses the model's response back into one of the candidate
/// ids (or `none`).
///
/// **Failure semantics** (normative per the Phase 4 stage brief):
///
/// | Failure mode             | `reason` tag       | Outcome                  |
/// |--------------------------|--------------------|--------------------------|
/// | call exceeded the timeout| `timeout`          | [`SkillSelection::None`] |
/// | runner returned `Err(_)` | `runner_error`     | [`SkillSelection::None`] |
/// | `RunResult.error.is_some`| `upstream_error`   | [`SkillSelection::None`] |
/// | response could not be parsed back to any candidate id | `parse_error` | [`SkillSelection::None`] |
/// | response named a skill id not in the candidate list   | `unknown_id`  | [`SkillSelection::None`] |
///
/// Every failure emits a WARN-level structured log on the
/// `skill_selector_failed_total` target with a `reason` field — this
/// is the canonical metric surface for v1. A successful selection or
/// a model-declined `none` is INFO, never WARN. One tracing span per
/// outcome carries `skill.selector.outcome` and (on failure)
/// `skill.selector.reason`.
pub struct LlmSkillSelector {
    runner: Arc<dyn AiRunner>,
    timeout: Duration,
    model: Option<String>,
}

impl LlmSkillSelector {
    /// Construct with defaults: 2 second timeout, Haiku model hint.
    pub fn new(runner: Arc<dyn AiRunner>) -> Self {
        Self {
            runner,
            timeout: DEFAULT_LLM_SELECTOR_TIMEOUT,
            model: Some(DEFAULT_LLM_SELECTOR_MODEL.to_string()),
        }
    }

    /// Start a builder when the defaults need tweaking (timeout or
    /// model id). Hosts that are happy with the defaults just call
    /// [`Self::new`].
    pub fn builder(runner: Arc<dyn AiRunner>) -> LlmSkillSelectorBuilder {
        LlmSkillSelectorBuilder {
            runner,
            timeout: DEFAULT_LLM_SELECTOR_TIMEOUT,
            model: Some(DEFAULT_LLM_SELECTOR_MODEL.to_string()),
        }
    }
}

impl std::fmt::Debug for LlmSkillSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmSkillSelector")
            .field("timeout", &self.timeout)
            .field("model", &self.model)
            .finish()
    }
}

/// Builder for [`LlmSkillSelector`].
pub struct LlmSkillSelectorBuilder {
    runner: Arc<dyn AiRunner>,
    timeout: Duration,
    model: Option<String>,
}

impl LlmSkillSelectorBuilder {
    /// Override the per-call hard timeout. The default is
    /// [`DEFAULT_LLM_SELECTOR_TIMEOUT`] (2 s). There are no retries —
    /// the call either completes inside the budget or the selector
    /// falls back to [`SkillSelection::None`].
    pub fn with_timeout(mut self, t: Duration) -> Self {
        self.timeout = t;
        self
    }

    /// Override the model id passed to the runner. Set to `None` to
    /// let the runner pick its own default.
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }

    /// Finalise the selector.
    pub fn build(self) -> LlmSkillSelector {
        LlmSkillSelector {
            runner: self.runner,
            timeout: self.timeout,
            model: self.model,
        }
    }
}

/// Build the prompt the model sees. Pinned in a free function so a
/// test can spot-check the surface without spinning a runner.
///
/// Determinism: candidates are iterated in the order the registry
/// passed them in — a `BTreeMap` by `SkillId`, lexicographic.
fn build_selector_prompt(candidates: &[Arc<Skill>]) -> String {
    use std::fmt::Write;
    let mut s = String::new();
    s.push_str(
        "Select the single best skill for the user's request from the list below.\n\
         Respond with ONLY the skill id (e.g. `starter.example.greet`) or the literal word `none` \
         if no skill matches.\n\n\
         Skills:\n",
    );
    for c in candidates {
        // `writeln!` into a String is infallible.
        let _ = writeln!(&mut s, "- {} — {}", c.id, c.description);
    }
    s
}

/// Translate a `Skill` reference into a [`SkillSelection::Selected`].
fn skill_to_selection(s: &Skill) -> SkillSelection {
    SkillSelection::Selected {
        skill_id: s.id.clone(),
        allowed_tools: s.allowed_tools.clone(),
        resources: s.resources.clone(),
        content_hash: s.bundle_hash.clone(),
    }
}

/// Static no-op `Cancel` handed to `AiRunner::run`. The outer
/// `tokio::time::timeout` is what enforces the budget — the runner
/// never sees cancellation from this selector.
struct NoOpCancel;
impl Cancel for NoOpCancel {
    fn is_cancelled(&self) -> bool {
        false
    }
    fn cancelled<'a>(
        &'a self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

/// Emit the WARN + metric line that every failure mode of
/// [`LlmSkillSelector`] funnels through. Centralised so a test can
/// grep one target.
fn record_llm_failure(reason: &'static str) {
    tracing::warn!(
        target: "skill_selector_failed_total",
        reason = reason,
        "skill selector fell back to None"
    );
    tracing::warn!(
        target: "skills.selector",
        outcome = "none",
        reason = reason,
        "skill selector failure"
    );
}

#[async_trait]
impl SelectorStrategy for LlmSkillSelector {
    async fn select_from(
        &self,
        candidates: &[Arc<Skill>],
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        if candidates.is_empty() {
            tracing::debug!(
                target: "skills.selector",
                outcome = "none",
                "no candidates"
            );
            return Ok(SkillSelection::None);
        }

        let prompt = build_selector_prompt(candidates);
        let cfg = RestCfg {
            prompt,
            model: self.model.clone(),
            ..RestCfg::default()
        };
        // Bounded channel so a runner that streams 100s of events
        // can't starve memory while we wait on the final result.
        let (tx, mut rx) = mpsc::channel::<Event>(8);
        let drain = tokio::spawn(async move { while rx.recv().await.is_some() {} });
        let session_id: SessionId = "skill-selector".into();
        let cancel = NoOpCancel;

        let run = self.runner.run(RunnerInput::Rest(cfg), session_id, tx, &cancel);
        let outcome = tokio::time::timeout(self.timeout, run).await;
        drain.abort();

        let rr = match outcome {
            Err(_elapsed) => {
                record_llm_failure("timeout");
                return Ok(SkillSelection::None);
            }
            Ok(Err(_runner_err)) => {
                record_llm_failure("runner_error");
                return Ok(SkillSelection::None);
            }
            Ok(Ok(rr)) => rr,
        };

        if rr.error.is_some() {
            record_llm_failure("upstream_error");
            return Ok(SkillSelection::None);
        }

        // Parse the response: first non-empty trimmed line, lowercased
        // for the `none` comparison, but matched **case-sensitively**
        // against candidate ids (skill ids are reverse-DNS, lower-case
        // by convention, but we want a strict match either way).
        let raw = rr.text.trim();
        let picked = raw
            .lines()
            .find_map(|line| {
                let l = line.trim();
                if l.is_empty() {
                    None
                } else {
                    Some(l)
                }
            })
            .unwrap_or("");

        if picked.is_empty() {
            record_llm_failure("parse_error");
            return Ok(SkillSelection::None);
        }
        if picked.eq_ignore_ascii_case("none") {
            tracing::info!(
                target: "skills.selector",
                outcome = "none_chosen",
                "model declined to select a skill"
            );
            return Ok(SkillSelection::None);
        }

        match candidates.iter().find(|c| c.id.as_str() == picked) {
            Some(s) => {
                tracing::info!(
                    target: "skills.selector",
                    outcome = "selected",
                    skill_id = %s.id,
                    "skill selector picked a skill"
                );
                Ok(skill_to_selection(s))
            }
            None => {
                record_llm_failure("unknown_id");
                Ok(SkillSelection::None)
            }
        }
    }
}

// ---------------------------------------------------------------------
// KeywordSkillSelector
// ---------------------------------------------------------------------

/// Deterministic keyword-match selector. No LLM call.
///
/// Picks the alphabetically-first approved skill whose description
/// tokens overlap any string value in `input`. If no candidate
/// overlaps, returns the alphabetically-first candidate as a stable
/// fallback so a flow that depends on *some* skill being selected
/// still gets one. Hosts that want stricter behaviour (no overlap →
/// no selection) can swap in their own [`SelectorStrategy`].
///
/// "Alphabetically-first" means first in the candidate slice the
/// registry passed in, which is `BTreeMap<SkillId, _>` order —
/// lexicographic by reverse-DNS id.
#[derive(Debug, Default, Clone, Copy)]
pub struct KeywordSkillSelector;

impl KeywordSkillSelector {
    /// Construct. Selector is stateless.
    pub fn new() -> Self {
        Self
    }
}

fn tokens_of(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
}

fn input_corpus(input: &SlotMap) -> String {
    use starter_flow_spi::node::SlotValue;
    let mut out = String::new();
    // SlotMap iteration order is stable across calls; we want a
    // deterministic corpus so two callers with the same input get the
    // same match.
    for (_slot_name, value) in input.iter() {
        if let SlotValue::String(s) = value {
            out.push(' ');
            out.push_str(s);
        }
    }
    out
}

#[async_trait]
impl SelectorStrategy for KeywordSkillSelector {
    async fn select_from(
        &self,
        candidates: &[Arc<Skill>],
        input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        if candidates.is_empty() {
            return Ok(SkillSelection::None);
        }
        let corpus = input_corpus(input);
        let input_tokens: std::collections::HashSet<String> = tokens_of(&corpus).collect();

        // First candidate (in BTreeMap order) whose description
        // tokens overlap any input token wins.
        for c in candidates {
            if tokens_of(&c.description).any(|t| input_tokens.contains(&t)) {
                tracing::info!(
                    target: "skills.selector",
                    outcome = "selected",
                    skill_id = %c.id,
                    "keyword selector matched"
                );
                return Ok(skill_to_selection(c));
            }
        }
        // Fallback: deterministic first candidate so flows that
        // expect *some* skill always get the same one.
        let first = &candidates[0];
        tracing::info!(
            target: "skills.selector",
            outcome = "selected",
            skill_id = %first.id,
            "keyword selector fell back to first candidate"
        );
        Ok(skill_to_selection(first))
    }
}

// ---------------------------------------------------------------------
// FirstSkillSelector
// ---------------------------------------------------------------------

/// Always picks the alphabetically-first approved skill. A test
/// fixture, not a recommended production strategy.
#[derive(Debug, Default, Clone, Copy)]
pub struct FirstSkillSelector;

impl FirstSkillSelector {
    /// Construct. Selector is stateless.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl SelectorStrategy for FirstSkillSelector {
    async fn select_from(
        &self,
        candidates: &[Arc<Skill>],
        _input: &SlotMap,
        _principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        Ok(candidates
            .first()
            .map(|s| skill_to_selection(s))
            .unwrap_or(SkillSelection::None))
    }
}

// ---------------------------------------------------------------------
// SkillRegistry: SkillSelector
// ---------------------------------------------------------------------

#[async_trait]
impl SkillSelector for SkillRegistry {
    async fn select(
        &self,
        input: &SlotMap,
        principal: &Principal,
    ) -> Result<SkillSelection, SkillError> {
        // R-skills-3 invariant: quarantined bundles must never reach
        // the strategy. `self.list()` is approved-only by construction.
        let candidates = self.list();
        let strategy = self.strategy();
        strategy.select_from(&candidates, input, principal).await
    }
}
