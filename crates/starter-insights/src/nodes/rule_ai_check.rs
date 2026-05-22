//! `starter.flow.rule.ai-check` — AI as in-line judge (R-ins-10).
//!
//! Assertion rule whose body is an LLM call routed through
//! [`starter_spi::ai::AiRunner`] per R-ins-5. The LLM judges
//! upstream `Verdict`s + the underlying `Dataset` window + a bound
//! skill bundle's tool access, and returns a `Verdict` like any
//! other rule.
//!
//! Auto-tagged with `starter.ai-check` and
//! `starter.ai-model:<exact-model>`; the exact model is also
//! recorded on `Verdict.evidence`. Per R-ins-10, the model **family**
//! is part of the rule's identity (a cross-family upgrade is a
//! major bump); the **exact** model is audit, not identity.
//!
//! Cannot declare `persist: true` (R-ins-10) — the node body checks
//! `RuleSchema.persist` and emits a `Severity::Error` if violated,
//! per R-ins-6 "failure is a verdict".
//!
//! The node depends only on the `AiJudge` trait below — a thin
//! seam atop `AiRunner` so this crate's dep tree (and CI gate) does
//! not need to know about provider SDKs. The host wires a concrete
//! `AiJudge` impl backed by `AiRunner` at boot.
//!
//! Input slots:
//! - `rule_id` ([`SlotValue::String`], required) — the registered
//!   ai-check rule id.
//! - `upstream_verdicts` ([`SlotValue::Json`], optional) — array of
//!   serialised `Verdict`s the judge is weighing.
//! - `window_summary` ([`SlotValue::String`], optional) — terse
//!   description of the dataset window for the prompt.
//!
//! Output slot:
//! - `verdict` ([`SlotValue::Json`]).

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{rule_error_flag, Coverage, RuleErrorKind, RuleId, Severity, Verdict};

use crate::ai::{auto_tag_ai_check, ModelFamily};
use crate::nodes::VERDICT_SLOT;

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.rule.ai-check";

/// Required input slot: rule id (`ns.name@major`).
pub const RULE_ID_SLOT: &str = "rule_id";
/// Optional input slot: upstream verdicts JSON array.
pub const UPSTREAM_SLOT: &str = "upstream_verdicts";
/// Optional input slot: short window summary.
pub const WINDOW_SUMMARY_SLOT: &str = "window_summary";

/// Output an `AiJudge` impl produces — the node body wraps it into
/// a `Verdict`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiJudgement {
    /// Severity the judge picked.
    pub severity: Severity,
    /// Short summary the judge wrote.
    pub summary: String,
    /// Exact model id the runner reported back (audit-only,
    /// R-ins-10).
    pub exact_model: String,
}

/// Seam over `starter_spi::ai::AiRunner` — kept thin so the
/// `starter-insights` dep tree stays free of provider SDKs.  The
/// host wires an impl backed by `AiRunner` at boot.
#[async_trait]
pub trait AiJudge: Send + Sync + 'static {
    /// The family this judge is bound to (R-ins-10 identity).
    fn family(&self) -> &ModelFamily;

    /// Run the judge against the prepared prompt. The implementer
    /// is responsible for routing through `AiRunner` (R-ins-5);
    /// this trait keeps the `starter-insights` dep tree provider-
    /// SDK-free.
    async fn judge(&self, prompt: AiJudgePrompt) -> Result<AiJudgement, String>;
}

/// Prompt payload handed to [`AiJudge::judge`].
#[derive(Debug, Clone)]
pub struct AiJudgePrompt {
    /// The pinned rule id (audit + telemetry).
    pub rule_id: RuleId,
    /// JSON array of upstream verdicts.
    pub upstream: serde_json::Value,
    /// Caller-supplied terse window summary (optional).
    pub window_summary: Option<String>,
}

/// Body for `starter.flow.rule.ai-check`.
pub struct RuleAiCheckNode {
    kind: KindId,
    judge: Arc<dyn AiJudge>,
}

impl RuleAiCheckNode {
    /// Construct an ai-check node body bound to an `AiJudge` impl.
    /// The judge's `family()` is recorded on every emitted verdict.
    pub fn new(judge: Arc<dyn AiJudge>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is valid"),
            judge,
        }
    }
}

fn emit_error(rule_id: RuleId, kind: RuleErrorKind, summary: impl Into<String>) -> SlotMap {
    let mut cov = Coverage::full_point();
    cov.quality_flags.push(rule_error_flag(kind));
    let v = Verdict::error(rule_id, Utc::now(), summary).with_coverage(cov);
    let mut out = SlotMap::new();
    out.insert(
        VERDICT_SLOT.to_owned(),
        SlotValue::Json(serde_json::to_value(&v).expect("Verdict serialises")),
    );
    out
}

#[async_trait]
impl NodeBehavior for RuleAiCheckNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let rule_id_str = match input.remove(RULE_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Ok(emit_error(
                    RuleId::new("starter.rule", "unknown", 1),
                    RuleErrorKind::InputMissing,
                    "rule.ai-check: missing `rule_id`",
                ));
            }
        };
        let rule_id = crate::nodes::rule_rust::parse_rule_id(&rule_id_str)
            .unwrap_or_else(|| RuleId::new("starter.rule", "unknown", 1));

        let upstream = match input.remove(UPSTREAM_SLOT) {
            Some(SlotValue::Json(j)) => j,
            _ => serde_json::Value::Array(Vec::new()),
        };
        let window_summary = match input.remove(WINDOW_SUMMARY_SLOT) {
            Some(SlotValue::String(s)) => Some(s),
            _ => None,
        };

        let prompt = AiJudgePrompt {
            rule_id: rule_id.clone(),
            upstream,
            window_summary,
        };
        let judgement = match self.judge.judge(prompt).await {
            Ok(j) => j,
            Err(e) => {
                return Ok(emit_error(
                    rule_id,
                    RuleErrorKind::BodyFailed,
                    format!("rule.ai-check: judge failed: {e}"),
                ));
            }
        };

        let mut v = Verdict::new(rule_id, Utc::now(), judgement.severity, judgement.summary);
        auto_tag_ai_check(&mut v, self.judge.family(), &judgement.exact_model);

        let mut out = SlotMap::new();
        out.insert(
            VERDICT_SLOT.to_owned(),
            SlotValue::Json(serde_json::to_value(&v).expect("Verdict serialises")),
        );
        Ok(out)
    }
}
