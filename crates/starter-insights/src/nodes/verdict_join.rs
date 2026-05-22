//! `starter.flow.verdict.join` — fan-in node combining N `Verdict`
//! inputs into one (R-ins-6 join semantics).
//!
//! Modes:
//! - `all`: severity = max non-error; any `Error` propagates as
//!   `Error` (the join cannot claim health it doesn't have).
//! - `any`: fires if any input is `>= Warn`; `Error` propagates as
//!   `Error`, not as `Warn`.
//! - `weighted`: each input has a `weight: f32` declared at the
//!   pipeline node. `Error` inputs are excluded and their weight is
//!   redistributed proportionally across non-error inputs.
//!
//! Degenerate cases:
//! - **All inputs errored** → `Severity::Error`,
//!   `effective.confidence = 0.0`, flag
//!   `starter.quality.join-all-inputs-errored@1` plus each input's
//!   `RuleError` flag.
//! - **Zero inputs** → engine rejects at flow validation time
//!   (per flow R3). Phase 1 still surfaces the case at runtime as
//!   an `Error` verdict so a misconfigured smoke is observable.
//! - **Single input** → pass-through, with the join's `rule_id`
//!   stamped on so the joined verdict stays addressable.

use async_trait::async_trait;
use chrono::Utc;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    join_all_inputs_errored_flag, Coverage, EffectiveCoverage, RawCoverage, RuleId, Severity, Tags,
    Verdict,
};

use super::VERDICT_SLOT;

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.verdict.join";

/// Body for `starter.flow.verdict.join`. Stateless — the join id is
/// configured at construction time (one body per pipeline-node
/// "this is the joined rule" identity per R-ins-6).
pub struct VerdictJoinNode {
    kind: KindId,
    /// Synthetic id stamped on the joined verdict.
    join_id: RuleId,
    /// Join mode.
    mode: JoinMode,
}

/// Supported join modes.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum JoinMode {
    /// `all` — max-severity, error propagates.
    All,
    /// `any` — fires at `>= Warn`; error propagates.
    Any,
    /// `weighted` — `(input_slot_name, weight)` pairs. Error
    /// inputs are excluded; their weight is redistributed
    /// proportionally across non-error inputs.
    Weighted(Vec<(String, f32)>),
}

impl VerdictJoinNode {
    /// Construct a join body. `join_id` is the synthetic
    /// `(pipeline_namespace, pipeline_name, semver)` rule id
    /// stamped on the joined verdict.
    pub fn new(join_id: RuleId, mode: JoinMode) -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is a valid reverse-DNS id"),
            join_id,
            mode,
        }
    }
}

#[async_trait]
impl NodeBehavior for VerdictJoinNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // Collect every input slot that decodes as a Verdict.
        let mut inputs: Vec<(String, Verdict)> = Vec::new();
        let mut bad_slots: Vec<String> = Vec::new();
        for (k, v) in input.into_iter() {
            match v {
                SlotValue::Json(j) => match serde_json::from_value::<Verdict>(j) {
                    Ok(verdict) => inputs.push((k, verdict)),
                    Err(_) => bad_slots.push(k),
                },
                _ => bad_slots.push(k),
            }
        }

        if inputs.is_empty() {
            // Zero-input degenerate (engine should reject; runtime
            // safety net).
            let now = Utc::now();
            let mut cov = Coverage::full_point();
            cov.effective.confidence = 0.0;
            cov.quality_flags.push(join_all_inputs_errored_flag());
            let v = Verdict::new(
                self.join_id.clone(),
                now,
                Severity::Error,
                "verdict.join: zero inputs",
            )
            .with_coverage(cov);
            return Ok(into_slot_map(&v));
        }

        let now = Utc::now();
        let all_errored = inputs.iter().all(|(_, v)| v.severity == Severity::Error);

        // Build the joined coverage.
        let mut joined_cov = build_joined_coverage(&inputs);

        let (severity, summary) = if all_errored {
            joined_cov
                .quality_flags
                .push(join_all_inputs_errored_flag());
            joined_cov.effective.confidence = 0.0;
            (
                Severity::Error,
                format!("verdict.join({} all-Error)", inputs.len()),
            )
        } else {
            match &self.mode {
                JoinMode::All => sev_all(&inputs),
                JoinMode::Any => sev_any(&inputs),
                JoinMode::Weighted(weights) => sev_weighted(&inputs, weights),
            }
        };

        // Union tags from all inputs (truncating at the cap).
        let mut joined_tags = Tags::empty();
        let mut truncated_seen = false;
        for (_, v) in &inputs {
            let (merged, t) = joined_tags.merge(v.tags.clone());
            joined_tags = merged;
            truncated_seen |= t;
        }
        if truncated_seen {
            joined_cov
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

        let verdict = Verdict::new(self.join_id.clone(), now, severity, summary)
            .with_coverage(joined_cov)
            .with_tags(joined_tags);
        Ok(into_slot_map(&verdict))
    }
}

fn sev_all(inputs: &[(String, Verdict)]) -> (Severity, String) {
    if inputs.iter().any(|(_, v)| v.severity == Severity::Error) {
        return (
            Severity::Error,
            "verdict.join(all): at least one input errored".to_owned(),
        );
    }
    let max = inputs
        .iter()
        .map(|(_, v)| v.severity)
        .max_by_key(|s| s.rank())
        .unwrap_or(Severity::Healthy);
    (
        max,
        format!("verdict.join(all,n={}): {:?}", inputs.len(), max),
    )
}

fn sev_any(inputs: &[(String, Verdict)]) -> (Severity, String) {
    if inputs.iter().any(|(_, v)| v.severity == Severity::Error) {
        return (
            Severity::Error,
            "verdict.join(any): at least one input errored".to_owned(),
        );
    }
    let fires = inputs
        .iter()
        .any(|(_, v)| v.severity.rank() >= Severity::Warn.rank());
    let max = inputs
        .iter()
        .map(|(_, v)| v.severity)
        .max_by_key(|s| s.rank())
        .unwrap_or(Severity::Healthy);
    let final_sev = if fires { max } else { Severity::Healthy };
    (
        final_sev,
        format!("verdict.join(any,n={}): {:?}", inputs.len(), final_sev),
    )
}

fn sev_weighted(inputs: &[(String, Verdict)], weights: &[(String, f32)]) -> (Severity, String) {
    // Error inputs excluded; their weight redistributes proportionally.
    let mut total_weight = 0.0f32;
    let mut error_weight = 0.0f32;
    for (slot, w) in weights {
        if let Some((_, v)) = inputs.iter().find(|(k, _)| k == slot) {
            total_weight += w;
            if v.severity == Severity::Error {
                error_weight += w;
            }
        }
    }
    let surviving_weight = total_weight - error_weight;
    if surviving_weight <= 0.0 {
        return (
            Severity::Error,
            "verdict.join(weighted): every weighted input errored".to_owned(),
        );
    }
    let scale = total_weight / surviving_weight;
    let mut score = 0.0f32;
    for (slot, w) in weights {
        if let Some((_, v)) = inputs.iter().find(|(k, _)| k == slot) {
            if v.severity == Severity::Error {
                continue;
            }
            score += (w * scale) * v.severity.rank() as f32;
        }
    }
    // Normalise to per-input rank.
    let mean_rank = score / total_weight;
    let final_sev = if mean_rank >= Severity::Critical.rank() as f32 {
        Severity::Critical
    } else if mean_rank >= Severity::Warn.rank() as f32 {
        Severity::Warn
    } else if mean_rank >= Severity::Info.rank() as f32 {
        Severity::Info
    } else {
        Severity::Healthy
    };
    (
        final_sev,
        format!(
            "verdict.join(weighted,n={}): mean_rank={:.2} -> {:?}",
            inputs.len(),
            mean_rank,
            final_sev
        ),
    )
}

fn build_joined_coverage(inputs: &[(String, Verdict)]) -> Coverage {
    // raw = union of inputs' raw (sum samples_expected / samples_present;
    // confidence = min of inputs' non-error raw confidences).
    let mut samples_expected = 0u64;
    let mut samples_present = 0u64;
    let mut min_raw_conf = 1.0f32;
    let mut effective_conf = 1.0f32;
    let mut penalty_chain = Vec::new();
    let mut flags = Vec::new();

    for (_, v) in inputs {
        samples_expected = samples_expected.saturating_add(v.coverage.raw.samples_expected);
        samples_present = samples_present.saturating_add(v.coverage.raw.samples_present);
        if v.severity != Severity::Error {
            min_raw_conf = min_raw_conf.min(v.coverage.raw.confidence);
            effective_conf = effective_conf.min(v.coverage.effective.confidence);
        }
        for entry in &v.coverage.effective.penalty_chain {
            penalty_chain.push(entry.clone());
        }
        for f in &v.coverage.quality_flags {
            flags.push(f.clone());
        }
    }

    Coverage::from_parts(
        RawCoverage::new(samples_expected, samples_present, min_raw_conf),
        EffectiveCoverage::from_parts(effective_conf, penalty_chain),
        flags,
    )
}

fn into_slot_map(v: &Verdict) -> SlotMap {
    let mut out = SlotMap::new();
    out.insert(
        VERDICT_SLOT.to_owned(),
        SlotValue::Json(serde_json::to_value(v).expect("Verdict serialises")),
    );
    out
}
