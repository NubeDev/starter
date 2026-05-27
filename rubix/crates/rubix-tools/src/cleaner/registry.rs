//! Ordered registry of [`AnomalyRule`]s.
//!
//! Rules run in registration order; the **first non-Ok outcome
//! wins** (see [`super`] module docs). The registry is just a
//! `Vec<Arc<dyn AnomalyRule>>` with two thin verbs (`add` /
//! `apply_all`) plus a `builtin()` constructor that preloads the
//! three builtin rules in the order that lets each rule pull its
//! weight without stepping on others:
//!
//! 1. [`NanRule`] — short-circuits before numeric checks see NaN.
//! 2. [`SpikeRule`] — needs a finite `last` to compare.
//! 3. [`StuckRule`] — walks the tail of `window.history`.
//!
//! Extension rules (`#3b`) wrap into the same trait and `add`
//! after the builtins so a manifest grant cannot accidentally
//! shadow `NanRule`.

use std::sync::Arc;

use super::builtin::{NanRule, SpikeRule, StuckRule};
use super::rule::{AnomalyRule, Reading, RuleOutcome, WindowSlice};

/// Ordered collection of rules.
///
/// Cheap to clone; rules behind `Arc`. Construction is two-step:
/// `RuleRegistry::new().add(rule).add(other)`. For the common case
/// of "all builtins in default order", use [`Self::builtin`].
#[derive(Clone, Default)]
pub struct RuleRegistry {
    rules: Vec<Arc<dyn AnomalyRule>>,
}

impl std::fmt::Debug for RuleRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleRegistry")
            .field("count", &self.rules.len())
            .field(
                "ids",
                &self.rules.iter().map(|r| r.id()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl RuleRegistry {
    /// Empty registry. Prefer [`Self::builtin`] for the default
    /// shape; this is for tests and for adapters that need
    /// extension-only rule sets.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-populated with the three builtin rules in their
    /// canonical order (NaN → Spike → Stuck). Extensions add
    /// after via [`Self::add`].
    pub fn builtin() -> Self {
        Self::new()
            .add(Arc::new(NanRule))
            .add(Arc::new(SpikeRule::default()))
            .add(Arc::new(StuckRule::default()))
    }

    /// Append a rule. Builder-style.
    pub fn add(mut self, rule: Arc<dyn AnomalyRule>) -> Self {
        self.rules.push(rule);
        self
    }

    /// Number of registered rules. Surfaced so the cleaner can log
    /// it once per tick and operators can spot drift between boots.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// `true` if the registry is empty (no rules — every row
    /// passes through as `Ok`).
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Iterate over registered rule ids. Order matches application
    /// order. Useful for `info!` boot logs.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.rules.iter().map(|r| r.id())
    }

    /// Apply every rule to `row` and return the first non-`Ok`
    /// outcome (with the rule id that produced it) or `(None, Ok)`
    /// if every rule passed.
    ///
    /// The `&'static str` rule id is surfaced so the cleaner can
    /// log "row tagged `spike` by `builtin.spike`" without
    /// reaching back into the registry.
    pub fn apply_all(
        &self,
        row: &Reading,
        window: WindowSlice<'_>,
    ) -> (Option<&'static str>, RuleOutcome) {
        for rule in &self.rules {
            match rule.apply(row, window) {
                RuleOutcome::Ok => continue,
                other => return (Some(rule.id()), other),
            }
        }
        (None, RuleOutcome::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleaner::rule::QualityTag;

    fn r(ts_ms: i64, value: Option<f64>) -> Reading {
        Reading {
            tenant_id: "t".into(),
            entity_id: "e".into(),
            ts_ms,
            value,
            source_quality: 0,
        }
    }

    #[test]
    fn builtin_registers_three_rules_in_order() {
        let reg = RuleRegistry::builtin();
        assert_eq!(reg.len(), 3);
        let ids: Vec<_> = reg.ids().collect();
        assert_eq!(ids, vec!["builtin.nan", "builtin.spike", "builtin.stuck"]);
    }

    #[test]
    fn empty_registry_passes_every_row() {
        let reg = RuleRegistry::new();
        let row = r(10, Some(1.0));
        let (id, out) = reg.apply_all(&row, WindowSlice::new(&[]));
        assert!(id.is_none());
        assert!(matches!(out, RuleOutcome::Ok));
    }

    #[test]
    fn nan_fires_before_spike_because_of_order() {
        // A NaN row also has no previous-row baseline; even with
        // history, SpikeRule defers to NanRule. Confirm the
        // registry surfaces "builtin.nan" not "builtin.spike".
        let reg = RuleRegistry::builtin();
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(f64::NAN));
        let (id, out) = reg.apply_all(&row, WindowSlice::new(&history));
        assert_eq!(id, Some("builtin.nan"));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Nan,
                ..
            }
        ));
    }

    #[test]
    fn spike_wins_when_first_non_ok() {
        let reg = RuleRegistry::builtin();
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(500.0)); // 50× — spike
        let (id, out) = reg.apply_all(&row, WindowSlice::new(&history));
        assert_eq!(id, Some("builtin.spike"));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Spike,
                ..
            }
        ));
    }

    #[test]
    fn stuck_wins_when_only_stuck_fires() {
        let reg = RuleRegistry::builtin();
        let history = vec![r(1, Some(5.0)), r(2, Some(5.0)), r(3, Some(5.0))];
        let row = r(4, Some(5.0));
        let (id, out) = reg.apply_all(&row, WindowSlice::new(&history));
        assert_eq!(id, Some("builtin.stuck"));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Stuck,
                ..
            }
        ));
    }

    #[test]
    fn all_rules_passing_returns_none_and_ok() {
        let reg = RuleRegistry::builtin();
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(11.0)); // small change, no repeat
        let (id, out) = reg.apply_all(&row, WindowSlice::new(&history));
        assert!(id.is_none());
        assert!(matches!(out, RuleOutcome::Ok));
    }
}
