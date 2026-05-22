//! AI rule kinds — `rule.ai-check` and `rule.ai-debug`
//! (Insights SCOPE R-ins-10).
//!
//! Two distinct surfaces, both routed through
//! [`starter_spi::ai::AiRunner`] per R-ins-5 (no provider SDK in the
//! `starter-insights` dep tree — the runner trait lives in
//! `starter-spi`, the concrete runners live in `starter-ai`).
//!
//! - **`rule.ai-check`** — assertion: the LLM is an in-line judge.
//!   Carries a stable `RuleId`, pins a **model family** in its
//!   schema, and records the **exact model** that ran on each
//!   `Verdict.evidence`. Verdicts are auto-tagged
//!   `starter.ai-check` + `starter.ai-model:<name>`.  Cannot
//!   `persist: true` (R-ins-10).
//! - **`rule.ai-debug`** — diagnostic: explains an upstream
//!   `Severity::Error` verdict. Never emits a Verdict; emits a
//!   `RuleErrorDiagnosis` slot value with `likely_cause`,
//!   `suggested_fix`, `confidence`, `summary`, and auto-tags the
//!   slot with `starter.ai-debug` + `starter.ai-model:<name>`.
//!
//! Both are flow-spi `NodeBehavior` impls (in `nodes::rule_ai_check`
//! and `nodes::rule_ai_debug`); this module ships the shared types
//! the nodes write into — `ModelFamily`, `RuleErrorDiagnosis`, and
//! the auto-tag helpers.

use serde::{Deserialize, Serialize};
use starter_spi::insights::{EvidenceRow, TagValue, Tags, Verdict};

/// `starter.ai-check` — auto-tag attached to every verdict an
/// `rule.ai-check` node emits.
pub const TAG_AI_CHECK: &str = "starter.ai-check";

/// `starter.ai-debug` — auto-tag attached to every diagnosis a
/// `rule.ai-debug` node emits.
pub const TAG_AI_DEBUG: &str = "starter.ai-debug";

/// `starter.ai-model:<name>` — auto-tag carrying the exact model
/// id that produced the output.  Per R-ins-10 the family is part
/// of the rule identity (changing it is a major bump); the exact
/// model is *audit metadata*, not identity, so it rides as a tag.
pub const TAG_AI_MODEL_PREFIX: &str = "starter.ai-model";

/// Model family pinned in `RuleSchema` (R-ins-10).
///
/// Crossing a family boundary (Claude ↔ GPT, Opus ↔ Sonnet) is
/// always a major bump because the behavioural envelope changes.
/// Provider patch-level deprecations (Claude 4.6 → 4.7) do NOT
/// force a major if the family is unchanged — the exact-model
/// audit trail covers the difference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ModelFamily {
    /// Provider id, e.g. `anthropic`, `openai`.
    pub provider: String,
    /// Family name, e.g. `claude-opus-4`, `gpt-4o`.
    pub family: String,
}

impl ModelFamily {
    /// Construct a [`ModelFamily`].
    pub fn new(provider: impl Into<String>, family: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            family: family.into(),
        }
    }

    /// Compact `<provider>/<family>` rendering used in tags / logs.
    pub fn slug(&self) -> String {
        format!("{}/{}", self.provider, self.family)
    }
}

/// Output value of `rule.ai-debug` (R-ins-10).
///
/// Never a `Verdict`; never gates an action; feeds `action.notify`
/// or drafts a revision via the `tuner` skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RuleErrorDiagnosis {
    /// Short human-friendly description of what the AI thinks went
    /// wrong.
    pub summary: String,
    /// Structured "what we think went wrong" tag.
    pub likely_cause: String,
    /// Suggested fix, free-form text. The tuner agent reads this
    /// when drafting a revision.
    pub suggested_fix: String,
    /// Confidence in the diagnosis in `[0.0, 1.0]`.
    pub confidence: f32,
    /// Family of the AI that produced the diagnosis (R-ins-10
    /// audit-not-identity).
    pub model_family: ModelFamily,
    /// Exact model id the runner reported back.
    pub exact_model: String,
}

impl RuleErrorDiagnosis {
    /// Construct a [`RuleErrorDiagnosis`].
    pub fn new(
        summary: impl Into<String>,
        likely_cause: impl Into<String>,
        suggested_fix: impl Into<String>,
        confidence: f32,
        model_family: ModelFamily,
        exact_model: impl Into<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            likely_cause: likely_cause.into(),
            suggested_fix: suggested_fix.into(),
            confidence: confidence.clamp(0.0, 1.0),
            model_family,
            exact_model: exact_model.into(),
        }
    }
}

/// Attach the `starter.ai-check` + `starter.ai-model:<exact>`
/// auto-tags and push an [`EvidenceRow`] recording the exact model
/// (per-Verdict, per R-ins-10 audit).
pub fn auto_tag_ai_check(verdict: &mut Verdict, family: &ModelFamily, exact_model: &str) {
    let mut tags = std::mem::take(&mut verdict.tags);
    tags.insert(TAG_AI_CHECK.to_owned(), TagValue::Flag);
    tags.insert(
        format!("{TAG_AI_MODEL_PREFIX}:{}", family.slug()),
        TagValue::Value(exact_model.to_owned()),
    );
    verdict.tags = tags;
    verdict.evidence.push(EvidenceRow::new(serde_json::json!({
        "kind": "ai-check.model",
        "family": family.slug(),
        "exact_model": exact_model,
    })));
}

/// Attach the `starter.ai-debug` + `starter.ai-model:<exact>`
/// auto-tags to a `Tags` bag the caller is building for the
/// diagnosis slot's downstream consumers.
pub fn auto_tag_ai_debug(tags: &mut Tags, family: &ModelFamily, exact_model: &str) {
    tags.insert(TAG_AI_DEBUG.to_owned(), TagValue::Flag);
    tags.insert(
        format!("{TAG_AI_MODEL_PREFIX}:{}", family.slug()),
        TagValue::Value(exact_model.to_owned()),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use starter_spi::insights::{RuleId, Severity, Verdict};

    #[test]
    fn auto_tag_ai_check_inserts_tags_and_evidence() {
        let family = ModelFamily::new("anthropic", "claude-opus-4");
        let mut v = Verdict::new(RuleId::new("t", "r", 1), Utc::now(), Severity::Warn, "x");
        auto_tag_ai_check(&mut v, &family, "claude-opus-4-20250514");
        assert!(v.tags.get(TAG_AI_CHECK).is_some());
        assert!(
            v.tags.0.keys().any(|k| k.starts_with(TAG_AI_MODEL_PREFIX)),
            "exact-model tag present"
        );
        assert!(
            v.evidence
                .iter()
                .any(|e| e.value["kind"] == "ai-check.model"),
            "exact-model evidence row pushed"
        );
    }
}
