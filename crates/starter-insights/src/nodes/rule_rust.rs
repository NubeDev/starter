//! `starter.flow.rule.rust` — dispatch a registered
//! [`starter_spi::insights::Rule`] by [`RuleId`] (R-ins-3).
//!
//! Input slots:
//! - `rule_id` ([`SlotValue::String`], required) — `ns.name@major`
//!   triple. Parsed via [`parse_rule_id`].
//! - `params`  ([`SlotValue::Json`], optional) — parameter map
//!   passed verbatim to the rule (R-ins-2 thresholds-as-inputs).
//!
//! Output slots:
//! - `verdict` ([`SlotValue::Json`]) — serialised [`Verdict`]. On
//!   any internal failure the body emits a `Severity::Error`
//!   verdict carrying a `starter.quality.rule-error@1` flag (R-ins-6
//!   "failure is a verdict, not an exception").

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    rule_error_flag, Coverage, RuleErrorKind, RuleId, RuleInput, RuleOutput, Severity, Tags,
    Verdict,
};

use crate::registry::RuleRegistry;

use super::VERDICT_SLOT;

/// Reverse-DNS kind id (R-ins-3, R10).
pub const KIND_ID: &str = "starter.flow.rule.rust";

/// Required input slot — `RuleId` triple as a `ns.name@major`
/// string.
pub const RULE_ID_SLOT: &str = "rule_id";

/// Optional input slot — JSON object of parameters.
pub const PARAMS_SLOT: &str = "params";

/// Body for the `starter.flow.rule.rust` node kind. Stateless per
/// flow R5: the only field is the registry handle.
pub struct RuleRustNode {
    kind: KindId,
    registry: Arc<RuleRegistry>,
}

impl RuleRustNode {
    /// Construct a node body bound to a [`RuleRegistry`].
    pub fn new(registry: Arc<RuleRegistry>) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            registry,
        }
    }
}

/// Parse `ns.name@major` into a [`RuleId`].
///
/// The namespace is the first dot-delimited segment; the name is
/// the remainder before `@`. So `iot.sensor.in-range@1` parses
/// as `RuleId { namespace = "iot", name = "sensor.in-range",
/// major = 1 }`. This matches the registration convention every
/// Phase 1 IoT rule uses ("first segment is the pack").
pub fn parse_rule_id(s: &str) -> Option<RuleId> {
    let (left, major) = s.rsplit_once('@')?;
    let (namespace, name) = left.split_once('.')?;
    let major: u32 = major.parse().ok()?;
    Some(RuleId::new(namespace, name, major))
}

#[async_trait]
impl NodeBehavior for RuleRustNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        // Synthesised id used when the input slot is missing — we
        // still emit a verdict (R-ins-6), tagged with a stable
        // `starter.rule.unknown` rule id so downstream consumers
        // can route on it.
        let unknown_id = RuleId::new("starter.rule", "unknown", 1);

        // 1. Read & parse rule_id.
        let rule_id_str = match input.remove(RULE_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Ok(emit_error_verdict(
                    unknown_id,
                    RuleErrorKind::InputMissing,
                    "rule.rust: missing `rule_id` input slot",
                ));
            }
        };
        let rule_id = match parse_rule_id(&rule_id_str) {
            Some(id) => id,
            None => {
                return Ok(emit_error_verdict(
                    unknown_id,
                    RuleErrorKind::InputMissing,
                    format!("rule.rust: malformed rule_id `{rule_id_str}`"),
                ));
            }
        };

        // 2. Dispatch via the registry.
        let rule = match self.registry.get(&rule_id) {
            Some(r) => r,
            None => {
                return Ok(emit_error_verdict(
                    rule_id.clone(),
                    RuleErrorKind::InputMissing,
                    format!("rule.rust: rule `{rule_id}` not registered"),
                ));
            }
        };

        // 3. Pull optional params.
        let params = match input.remove(PARAMS_SLOT) {
            None | Some(SlotValue::Null) => serde_json::Map::new(),
            Some(SlotValue::Json(serde_json::Value::Object(m))) => m,
            Some(SlotValue::Json(other)) => {
                return Ok(emit_error_verdict(
                    rule_id,
                    RuleErrorKind::InputMissing,
                    format!("rule.rust: `params` must be a JSON object; got {other}"),
                ));
            }
            Some(other) => {
                return Ok(emit_error_verdict(
                    rule_id,
                    RuleErrorKind::InputMissing,
                    format!("rule.rust: `params` must be SlotValue::Json; got {other:?}"),
                ));
            }
        };

        // 4. Evaluate. Catch unwind so a panicking rule body never
        //    short-circuits the pipeline (R-ins-6); convert to
        //    Severity::Error verdict.
        let rule_input = RuleInput::from_parts(params, None);
        let verdict = match rule.evaluate(rule_input).await {
            RuleOutput::Assertion(v) => v,
            // Derivation outputs through rule.rust are a wiring
            // error — rule.derive is the marker kind for those.
            // Convert to Severity::Error per R-ins-6.
            _ => {
                return Ok(emit_error_verdict(
                    rule_id,
                    RuleErrorKind::BodyFailed,
                    "rule.rust: assertion rule expected; rule returned a non-assertion output",
                ));
            }
        };

        // 5. Merge rule.schema().tags ∪ verdict.tags (rule wins
        //    over verdict per R-ins-8 pipeline-node-wins applies
        //    one layer up; Phase 1 has no pipeline-node layer
        //    yet, so we union here).
        let mut out_verdict = verdict;
        let static_tags = rule.schema().tags.clone();
        let (merged, truncated) = static_tags.merge(out_verdict.tags);
        out_verdict.tags = merged;
        if truncated {
            out_verdict
                .coverage
                .quality_flags
                .push(starter_spi::insights::QualityFlag::new(
                    starter_spi::insights::QualityFlagId::new(
                        "starter.quality",
                        "tags-truncated",
                        1,
                    ),
                    starter_spi::insights::QualityFlagSeverity::Info,
                ));
        }

        Ok(verdict_into_slot_map(&out_verdict))
    }
}

fn verdict_into_slot_map(v: &Verdict) -> SlotMap {
    let mut out = SlotMap::new();
    out.insert(
        VERDICT_SLOT.to_owned(),
        SlotValue::Json(serde_json::to_value(v).expect("Verdict serialises")),
    );
    out
}

fn emit_error_verdict(rule_id: RuleId, kind: RuleErrorKind, summary: impl Into<String>) -> SlotMap {
    let now = Utc::now();
    let mut cov = Coverage::full_point();
    cov.quality_flags.push(rule_error_flag(kind));
    let v = Verdict::new(rule_id, now, Severity::Error, summary)
        .with_coverage(cov)
        .with_tags(Tags::empty());
    verdict_into_slot_map(&v)
}
