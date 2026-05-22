//! The [`Rule`] trait, [`RuleId`], [`RuleOutput`], [`RuleSchema`].

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

use super::dataset::Dataset;
use super::tags::Tags;
use super::verdict::Verdict;

/// `(namespace, name, major)` registry identifier for a rule.
///
/// Stable across pipelines: a `RuleId` registered once is referenced
/// from any flow on this host. A breaking change requires a new
/// major; the registry rejects duplicate `(namespace, name, major)`
/// registrations (R-ins-2).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RuleId {
    /// Reverse-DNS namespace, e.g. `iot`, `energy`, `org.acme`.
    pub namespace: String,
    /// Rule name within the namespace, e.g. `device.online`.
    pub name: String,
    /// Major version. Breaking changes bump this.
    pub major: u32,
}

impl RuleId {
    /// Construct a [`RuleId`].
    pub fn new(namespace: impl Into<String>, name: impl Into<String>, major: u32) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            major,
        }
    }
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}@{}", self.namespace, self.name, self.major)
    }
}

/// Two output shapes a [`Rule`] may return (R-ins-7).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RuleOutput {
    /// Assertion rule: `Dataset -> Verdict`.
    Assertion(Verdict),
    /// Derivation rule: `Dataset -> Dataset`.
    Derivation(Dataset),
}

/// Static metadata describing a registered [`Rule`].
///
/// Phase 2 widens the schema with the derivation-related knobs the
/// engine needs to enforce R-ins-6's load-bearing invariants:
/// `confidence_penalty` (the multiplicative discount applied to
/// `effective.confidence` whenever a derivation rule emits a
/// `Dataset`), `retroactive` (whether the rule's inputs may mutate
/// — drives D5's per-window watermark + the
/// `starter.quality.retroactive-correction@1` flag), `idempotent`
/// (D6 idempotence contract for hand-unrolled multi-pass cleaning),
/// `persist` (derivation cache opt-in — wired in Phase 3), and
/// `max_operations` (per-rule Rhai operation budget override per
/// R-ins-4). Defaults are conservative: no penalty (`None`), not
/// retroactive, not idempotent (must be opted in), no persist, no
/// budget override.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct RuleSchema {
    /// The rule's stable id.
    pub id: RuleId,
    /// Static tags merged into every emitted `Verdict`/`Dataset`
    /// (per R-ins-8).
    pub tags: Tags,
    /// Whether this rule is an assertion (returns `Verdict`) or a
    /// derivation (returns `Dataset`). Pipeline wiring is
    /// type-checked at flow load time against this declaration.
    pub kind: RuleKind,
    /// Derivation confidence penalty in `[0.0, 1.0]` (R-ins-6).
    /// Multiplied into `effective.confidence` by the engine when the
    /// rule emits a `Dataset`. `None` is treated as "no penalty"
    /// (1.0). A value `> 1.0` is rejected at registry registration
    /// time — derivations may only lower or preserve confidence,
    /// never raise it.
    pub confidence_penalty: Option<f32>,
    /// Marks the rule's inputs as mutable downstream — drives D5's
    /// per-window rollup invalidation + the
    /// `starter.quality.retroactive-correction@1` flag.
    pub retroactive: bool,
    /// Declares the rule is idempotent on its own output (D6). The
    /// determinism smoke runs the rule twice and asserts equality.
    pub idempotent: bool,
    /// Opt-in for the derivation cache (Phase 3). Currently honoured
    /// only as metadata; the cache itself lands later.
    pub persist: bool,
    /// Per-rule override of the Rhai sandbox operation budget
    /// (R-ins-4). `None` means "use the sandbox default".
    pub max_operations: Option<u64>,
}

/// Declared output kind of a [`Rule`] — read off [`RuleSchema`] by
/// the engine to type-check pipeline wiring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleKind {
    /// `Dataset -> Verdict`.
    Assertion,
    /// `Dataset -> Dataset`.
    Derivation,
}

impl RuleSchema {
    /// Construct an assertion-rule schema.
    pub fn assertion(id: RuleId) -> Self {
        Self {
            id,
            tags: Tags::default(),
            kind: RuleKind::Assertion,
            confidence_penalty: None,
            retroactive: false,
            idempotent: false,
            persist: false,
            max_operations: None,
        }
    }

    /// Construct a derivation-rule schema.
    pub fn derivation(id: RuleId) -> Self {
        Self {
            id,
            tags: Tags::default(),
            kind: RuleKind::Derivation,
            confidence_penalty: None,
            retroactive: false,
            idempotent: false,
            persist: false,
            max_operations: None,
        }
    }

    /// Attach static tags. Builder shape.
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = tags;
        self
    }

    /// Set the derivation `confidence_penalty`. Caller is
    /// responsible for keeping it in `[0.0, 1.0]`; the registry
    /// validates at registration time.
    pub fn with_confidence_penalty(mut self, penalty: f32) -> Self {
        self.confidence_penalty = Some(penalty);
        self
    }

    /// Mark the rule as retroactive (D5).
    pub fn retroactive(mut self) -> Self {
        self.retroactive = true;
        self
    }

    /// Mark the rule as idempotent (D6).
    pub fn idempotent(mut self) -> Self {
        self.idempotent = true;
        self
    }

    /// Override the Rhai sandbox operation budget (R-ins-4).
    pub fn with_max_operations(mut self, ops: u64) -> Self {
        self.max_operations = Some(ops);
        self
    }
}

/// Input handed to [`Rule::evaluate`]. Phase 1 ships the canonical
/// fields; richer typed inputs (param maps, schemas) layer on in
/// later phases.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct RuleInput {
    /// Caller-supplied parameters (thresholds, lookback, etc.).
    /// Pipelines pass their wiring through here per R-ins-2:
    /// thresholds are inputs, never captured at rule construction.
    pub params: serde_json::Map<String, serde_json::Value>,
    /// The dataset window the rule is evaluating. `None` for
    /// point-in-time rules (Phase 1 IoT smoke uses this path).
    pub dataset: Option<Dataset>,
}

impl RuleInput {
    /// Empty input. Helper for tests / point-in-time rules.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Read a parameter from `params` as a borrowed JSON value.
    pub fn param(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.get(key)
    }

    /// Builder: construct a [`RuleInput`] from a params map and an
    /// optional dataset. Use this instead of struct-expression
    /// initialisation; the struct is `#[non_exhaustive]`.
    pub fn from_parts(
        params: serde_json::Map<String, serde_json::Value>,
        dataset: Option<Dataset>,
    ) -> Self {
        Self { params, dataset }
    }
}

/// A reusable analysis unit (R-ins-2).
///
/// Stateless: `Send + Sync + 'static`, never `&mut self`. Thresholds,
/// baselines, lookback windows live in [`RuleInput::params`] —
/// captured-at-construction state is a bug and the registry's
/// determinism smoke catches it.
///
/// **Failure is a verdict, not an exception** (R-ins-6). A rule that
/// cannot produce an opinion returns a `Verdict` with
/// [`super::Severity::Error`] and a [`super::RULE_ERROR`] quality
/// flag — never `panic!`, never `Err`. This trait reflects that:
/// `evaluate` returns [`RuleOutput`] directly.
#[async_trait]
pub trait Rule: Send + Sync + 'static {
    /// Static metadata for the rule.
    fn schema(&self) -> &RuleSchema;

    /// Evaluate the rule against an input. Phase 1 nodes invoke
    /// this synchronously per sample; later phases run it on a
    /// streaming window.
    async fn evaluate(&self, input: RuleInput) -> RuleOutput;
}
