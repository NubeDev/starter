//! `starter.flow.rule.rhai` — Rhai-scripted assertion rule under
//! the locked sandbox profile (Insights SCOPE R-ins-4 + R-ins-6).
//!
//! Input slots:
//! - `script` ([`SlotValue::String`], required) — the Rhai source.
//!   Must evaluate to either a Rhai integer (severity rank) or a
//!   Rhai object map `#{ severity: "healthy"|"info"|"warn"|"critical"|"error",
//!   summary: "..." }`.
//! - `rule_id` ([`SlotValue::String`], optional) — if absent, the
//!   node mints a D4 anonymous id `anon.<blake3-prefix>` over the
//!   canonicalised script.
//! - `params` ([`SlotValue::Json`], optional) — JSON object handed
//!   to the script under the `params` global.
//! - `max_operations` ([`SlotValue::Int`], optional) — per-rule
//!   operation budget override (R-ins-4). Pipeline-level config
//!   cannot raise this above the script's own override; this slot
//!   exists for the inline-rule case where the rule has no
//!   registry entry.
//!
//! Output slot:
//! - `verdict` — serialised [`Verdict`]. Failure → `Severity::Error`
//!   with `starter.quality.rule-error@1` flag (R-ins-6).

use async_trait::async_trait;
use chrono::Utc;
use rhai::{Dynamic, Scope};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    rule_error_flag, Coverage, RuleErrorKind, RuleId, Severity, Tags, Verdict,
};

use crate::rhai_sandbox::make_engine;

use super::VERDICT_SLOT;

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.rule.rhai";

/// Required input slot: the Rhai script source.
pub const SCRIPT_SLOT: &str = "script";

/// Optional input slot: explicit `RuleId` (`ns.name@major`).
pub const RULE_ID_SLOT: &str = "rule_id";

/// Optional input slot: JSON params handed to the script.
pub const PARAMS_SLOT: &str = "params";

/// Optional input slot: per-rule operation budget override.
pub const MAX_OPERATIONS_SLOT: &str = "max_operations";

/// Body for `starter.flow.rule.rhai`. Stateless — every invocation
/// builds a fresh `Engine` (cheap, but more importantly side-effect
/// free across rules).
pub struct RuleRhaiNode {
    kind: KindId,
}

impl RuleRhaiNode {
    /// Construct a new `rule.rhai` node body.
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is valid"),
        }
    }
}

impl Default for RuleRhaiNode {
    fn default() -> Self {
        Self::new()
    }
}

/// D4: derive an anonymous `RuleId` (`anon.<blake3-prefix>`) from
/// the canonicalised script body. Stable across runs of the same
/// script; salts on the script bytes alone (params live elsewhere).
pub fn anon_rule_id(script: &str) -> RuleId {
    let hash = blake3::hash(script.as_bytes());
    let hex = hash.to_hex();
    // 12 hex chars = 48 bits, plenty for a per-host inline-script
    // namespace; matches D4's "anon.<blake3-prefix>" pattern.
    let prefix: String = hex.chars().take(12).collect();
    RuleId::new("anon", prefix, 1)
}

#[async_trait]
impl NodeBehavior for RuleRhaiNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let script = match input.remove(SCRIPT_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Ok(emit_error(
                    RuleId::new("starter.rule", "unknown", 1),
                    RuleErrorKind::InputMissing,
                    "rule.rhai: missing `script` input slot",
                ));
            }
        };

        let rule_id = match input.remove(RULE_ID_SLOT) {
            Some(SlotValue::String(s)) => crate::nodes::rule_rust::parse_rule_id(&s)
                .unwrap_or_else(|| anon_rule_id(&script)),
            _ => anon_rule_id(&script),
        };

        let params = match input.remove(PARAMS_SLOT) {
            None | Some(SlotValue::Null) => serde_json::Value::Object(Default::default()),
            Some(SlotValue::Json(j)) => j,
            other => {
                return Ok(emit_error(
                    rule_id,
                    RuleErrorKind::InputMissing,
                    format!("rule.rhai: `params` must be SlotValue::Json; got {other:?}"),
                ));
            }
        };

        let max_ops = match input.remove(MAX_OPERATIONS_SLOT) {
            Some(SlotValue::Int(n)) if n > 0 => Some(n as u64),
            _ => None,
        };

        let engine = make_engine(max_ops);
        let mut scope = Scope::new();
        // Expose `params` to the script as a Dynamic value.
        scope.push("params", json_to_dynamic(&params));

        let result: Result<Dynamic, _> = engine.eval_with_scope(&mut scope, &script);
        let value = match result {
            Ok(v) => v,
            Err(err) => {
                let msg = format!("{err}");
                let kind = if msg.to_lowercase().contains("operation") {
                    RuleErrorKind::BudgetExhausted
                } else {
                    RuleErrorKind::BodyFailed
                };
                return Ok(emit_error(rule_id, kind, format!("rule.rhai: {msg}")));
            }
        };

        let (severity, summary) = parse_rhai_result(&value).unwrap_or((
            Severity::Error,
            format!(
                "rule.rhai: script must return an integer rank or a map with `severity`/`summary`; got {:?}",
                value.type_name()
            ),
        ));

        let mut verdict = Verdict::new(rule_id, Utc::now(), severity, summary);
        if severity == Severity::Error {
            let mut cov = Coverage::full_point();
            cov.quality_flags
                .push(rule_error_flag(RuleErrorKind::BodyFailed));
            verdict = verdict.with_coverage(cov);
        }
        verdict = verdict.with_tags(Tags::empty());
        Ok(verdict_into_slot_map(&verdict))
    }
}

fn parse_rhai_result(v: &Dynamic) -> Option<(Severity, String)> {
    if let Some(i) = v.clone().try_cast::<i64>() {
        return Some((rank_to_severity(i), format!("rule.rhai: rank={i}")));
    }
    if let Some(map) = v.clone().try_cast::<rhai::Map>() {
        let severity = map
            .get("severity")
            .and_then(|d| d.clone().into_immutable_string().ok())
            .map(|s| str_to_severity(s.as_str()))
            .unwrap_or(Severity::Error);
        let summary = map
            .get("summary")
            .and_then(|d| d.clone().into_immutable_string().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("rule.rhai: {severity:?}"));
        return Some((severity, summary));
    }
    None
}

fn rank_to_severity(r: i64) -> Severity {
    match r {
        0 => Severity::Healthy,
        1 => Severity::Info,
        2 => Severity::Warn,
        3 => Severity::Critical,
        _ => Severity::Error,
    }
}

fn str_to_severity(s: &str) -> Severity {
    match s.to_ascii_lowercase().as_str() {
        "healthy" => Severity::Healthy,
        "info" => Severity::Info,
        "warn" | "warning" => Severity::Warn,
        "critical" | "crit" => Severity::Critical,
        _ => Severity::Error,
    }
}

fn json_to_dynamic(v: &serde_json::Value) -> Dynamic {
    match v {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::UNIT
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            let v: rhai::Array = arr.iter().map(json_to_dynamic).collect();
            Dynamic::from(v)
        }
        serde_json::Value::Object(obj) => {
            let mut m = rhai::Map::new();
            for (k, val) in obj {
                m.insert(k.as_str().into(), json_to_dynamic(val));
            }
            Dynamic::from(m)
        }
    }
}

fn emit_error(rule_id: RuleId, kind: RuleErrorKind, summary: impl Into<String>) -> SlotMap {
    let mut cov = Coverage::full_point();
    cov.quality_flags.push(rule_error_flag(kind));
    let v = Verdict::error(rule_id, Utc::now(), summary)
        .with_coverage(cov)
        .with_tags(Tags::empty());
    verdict_into_slot_map(&v)
}

fn verdict_into_slot_map(v: &Verdict) -> SlotMap {
    let mut out = SlotMap::new();
    out.insert(
        VERDICT_SLOT.to_owned(),
        SlotValue::Json(serde_json::to_value(v).expect("Verdict serialises")),
    );
    out
}
