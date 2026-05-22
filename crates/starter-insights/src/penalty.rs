//! Engine-side `confidence_penalty` enforcement (Insights SCOPE
//! R-ins-6 derivation coverage mutation contract).
//!
//! Derivation rules declare a `confidence_penalty: Option<f32>` on
//! their [`RuleSchema`]; whenever a derivation emits a `Dataset`,
//! the engine — never the rule body — multiplies
//! `effective.confidence` by the penalty and appends
//! `(rule_id, penalty)` to `penalty_chain`. The registry already
//! rejects `penalty > 1.0` at registration time (see
//! [`crate::registry::RuleRegistry::register`]); this module
//! applies the penalty on the wire.

use starter_spi::insights::{Dataset, RuleSchema};

/// Apply a derivation rule's declared `confidence_penalty` to a
/// `Dataset` produced by that rule. No-op for assertions or
/// derivations that declared no penalty.
///
/// Returns the modified dataset for chaining; the input
/// `effective.confidence` is multiplied by `penalty` and clamped to
/// `[0.0, 1.0]`, and `(rule_id, penalty)` is pushed onto
/// `penalty_chain`. The function is the single load-bearing seam
/// for the contract; a rule body that touches `effective` directly
/// is caught by the determinism smoke (R-ins-2).
pub fn apply_derivation_penalty(mut ds: Dataset, schema: &RuleSchema) -> Dataset {
    let Some(p) = schema.confidence_penalty else {
        return ds;
    };
    let next = (ds.coverage.effective.confidence * p).clamp(0.0, 1.0);
    ds.coverage.effective.confidence = next;
    ds.coverage
        .effective
        .penalty_chain
        .push((schema.id.clone(), p));
    ds
}

#[cfg(test)]
mod tests {
    use super::*;
    use starter_spi::insights::{
        Coverage, Dataset, DatasetSchema, RawCoverage, RuleId, RuleSchema, TimeZoneId,
        VecDatasetRows,
    };
    use std::sync::Arc;

    fn ds(conf: f32) -> Dataset {
        let raw = RawCoverage::new(10, 10, conf);
        Dataset::from_parts(
            DatasetSchema::new(Vec::<String>::new()),
            Arc::new(VecDatasetRows::empty()),
            Coverage::from_raw(raw),
            TimeZoneId::utc(),
            None,
        )
    }

    #[test]
    fn no_penalty_is_passthrough() {
        let schema = RuleSchema::derivation(RuleId::new("t", "r", 1));
        let out = apply_derivation_penalty(ds(1.0), &schema);
        assert_eq!(out.coverage.effective.confidence, 1.0);
        assert!(out.coverage.effective.penalty_chain.is_empty());
    }

    #[test]
    fn penalty_lowers_effective_and_appends_chain() {
        let schema = RuleSchema::derivation(RuleId::new("t", "r", 1)).with_confidence_penalty(0.8);
        let out = apply_derivation_penalty(ds(1.0), &schema);
        assert!((out.coverage.effective.confidence - 0.8).abs() < f32::EPSILON);
        assert_eq!(out.coverage.effective.penalty_chain.len(), 1);
    }

    #[test]
    fn penalty_chains_multiplicatively() {
        let a = RuleSchema::derivation(RuleId::new("t", "a", 1)).with_confidence_penalty(0.9);
        let b = RuleSchema::derivation(RuleId::new("t", "b", 1)).with_confidence_penalty(0.8);
        let out = apply_derivation_penalty(apply_derivation_penalty(ds(1.0), &a), &b);
        let expected = 1.0f32 * 0.9 * 0.8;
        assert!((out.coverage.effective.confidence - expected).abs() < 1e-6);
        assert_eq!(out.coverage.effective.penalty_chain.len(), 2);
    }
}
