//! `starter.flow.rule.ai-debug` — AI diagnostic for failed rules
//! (Insights SCOPE R-ins-10).
//!
//! Placed downstream of a flow `branch(on=severity)` that routes
//! `Severity::Error` verdicts. Inspects the failing verdict + the
//! rule body + the inputs that were present, and emits a structured
//! [`RuleErrorDiagnosis`](crate::ai::RuleErrorDiagnosis). Never
//! emits a `Verdict`; never gates an action; can only feed
//! `action.notify` or the `tuner` skill's draft path.
//!
//! Input slots:
//! - `error_verdict` ([`SlotValue::Json`], required) — the upstream
//!   `Severity::Error` verdict.
//! - `body_excerpt` ([`SlotValue::String`], optional) — Rhai/SQL
//!   text or Rust schema string handed to the explainer.
//!
//! Output slots:
//! - `diagnosis` ([`SlotValue::Json`]) — serialised
//!   [`RuleErrorDiagnosis`].
//! - `tags` ([`SlotValue::Json`]) — auto-tag bag
//!   (`starter.ai-debug` + `starter.ai-model:<exact>`).

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{Tags, Verdict};

use crate::ai::{auto_tag_ai_debug, ModelFamily, RuleErrorDiagnosis};

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.rule.ai-debug";

/// Required input slot: the upstream `Severity::Error` verdict.
pub const ERROR_VERDICT_SLOT: &str = "error_verdict";

/// Optional input slot: rule body excerpt.
pub const BODY_EXCERPT_SLOT: &str = "body_excerpt";

/// Output slot: serialised [`RuleErrorDiagnosis`].
pub const DIAGNOSIS_SLOT: &str = "diagnosis";

/// Output slot: serialised auto-tag bag.
pub const TAGS_SLOT: &str = "tags";

/// Prompt payload an [`AiDebugger`] consumes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDebugPrompt {
    /// The failing verdict.
    pub error_verdict: Verdict,
    /// Body excerpt (Rhai/SQL/Rust schema) — short.
    pub body_excerpt: Option<String>,
}

/// Output an [`AiDebugger`] returns — the node body wraps it into a
/// `RuleErrorDiagnosis`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDebugResponse {
    /// Human-friendly summary.
    pub summary: String,
    /// Structured likely-cause label.
    pub likely_cause: String,
    /// Suggested fix (free-form).
    pub suggested_fix: String,
    /// Confidence in `[0, 1]`.
    pub confidence: f32,
    /// Exact model id the runner reported back.
    pub exact_model: String,
}

/// Seam over `AiRunner` for the debugger flavour. Mirror of
/// [`crate::nodes::rule_ai_check::AiJudge`].
#[async_trait]
pub trait AiDebugger: Send + Sync + 'static {
    /// Model family the debugger is pinned to.
    fn family(&self) -> &ModelFamily;

    /// Generate a diagnosis. Routes through `AiRunner` in
    /// production; tests pass a deterministic stub.
    async fn diagnose(&self, prompt: AiDebugPrompt) -> Result<AiDebugResponse, String>;
}

/// Body for `starter.flow.rule.ai-debug`.
pub struct RuleAiDebugNode {
    kind: KindId,
    debugger: Arc<dyn AiDebugger>,
}

impl RuleAiDebugNode {
    /// Construct an ai-debug node body bound to an `AiDebugger` impl.
    pub fn new(debugger: Arc<dyn AiDebugger>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is valid"),
            debugger,
        }
    }
}

#[async_trait]
impl NodeBehavior for RuleAiDebugNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let verdict_json = match input.remove(ERROR_VERDICT_SLOT) {
            Some(SlotValue::Json(j)) => j,
            _ => {
                return Err(NodeError::InvalidInput(
                    "rule.ai-debug: `error_verdict` slot must be a serialised Verdict".into(),
                ))
            }
        };
        let verdict: Verdict = serde_json::from_value(verdict_json)
            .map_err(|e| NodeError::InvalidInput(format!("rule.ai-debug: bad verdict: {e}")))?;
        let body_excerpt = match input.remove(BODY_EXCERPT_SLOT) {
            Some(SlotValue::String(s)) => Some(s),
            _ => None,
        };

        let prompt = AiDebugPrompt {
            error_verdict: verdict,
            body_excerpt,
        };
        let resp = self
            .debugger
            .diagnose(prompt)
            .await
            .map_err(|e| NodeError::Backend(format!("rule.ai-debug: {e}")))?;

        let diag = RuleErrorDiagnosis::new(
            resp.summary,
            resp.likely_cause,
            resp.suggested_fix,
            resp.confidence,
            self.debugger.family().clone(),
            resp.exact_model.clone(),
        );
        let mut tags = Tags::empty();
        auto_tag_ai_debug(&mut tags, self.debugger.family(), &resp.exact_model);

        let mut out = SlotMap::new();
        out.insert(
            DIAGNOSIS_SLOT.to_owned(),
            SlotValue::Json(serde_json::to_value(&diag).expect("diagnosis serialises")),
        );
        out.insert(
            TAGS_SLOT.to_owned(),
            SlotValue::Json(serde_json::to_value(&tags).expect("tags serialise")),
        );
        Ok(out)
    }
}
