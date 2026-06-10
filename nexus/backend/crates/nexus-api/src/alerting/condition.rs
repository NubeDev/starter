//! Resolve a multi-condition rule to a single breaching boolean.
//!
//! A rule is a list of conditions combined with AND/OR. Each condition reduces
//! its query result to one value and compares it against a threshold. This module
//! is the pure part: given each condition's already-evaluated outcome, it folds
//! them with the combinator into the single `breaching` the untouched state
//! machine consumes. The I/O (running each query) lives in the evaluator; the
//! fold lives here so it is unit-testable without a database.

pub use nexus_spi::dto::alert::AlertCondition as Condition;

use super::reduce::Reducer;

/// The reducer a condition uses, parsed from its stored string.
pub fn reducer_of(cond: &Condition) -> Reducer {
    Reducer::parse(&cond.reducer)
}

/// How the conditions of a rule are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    And,
    Or,
}

impl Combinator {
    /// Parse the stored string form. Defaults to `And`, the safer combinator
    /// (a typo cannot make a rule fire more eagerly than intended).
    pub fn parse(s: &str) -> Self {
        match s {
            "or" => Combinator::Or,
            _ => Combinator::And,
        }
    }

    /// The stored string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Combinator::And => "and",
            Combinator::Or => "or",
        }
    }
}

/// The result of evaluating one condition: whether it breached, and whether its
/// query returned data at all (so the no-data policy can act on it). A `breaching`
/// of `false` with `had_data == false` is the no-data case the policy resolves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionOutcome {
    pub breaching: bool,
    pub had_data: bool,
    /// The reduced value, for the notification template and the event record.
    pub value: Option<f64>,
}

/// Fold each condition's outcome with the combinator into the single breaching
/// boolean the state machine consumes. Empty (no conditions) is non-breaching.
/// AND requires every condition breaching; OR requires any.
pub fn combine(outcomes: &[ConditionOutcome], combinator: Combinator) -> bool {
    if outcomes.is_empty() {
        return false;
    }
    match combinator {
        Combinator::And => outcomes.iter().all(|o| o.breaching),
        Combinator::Or => outcomes.iter().any(|o| o.breaching),
    }
}

/// Whether *any* condition lacked data — the trigger for the no-data policy. A
/// rule with one empty condition is "no data" even if another condition had rows,
/// because a missing input makes the combined result undefined.
pub fn any_no_data(outcomes: &[ConditionOutcome]) -> bool {
    outcomes.iter().any(|o| !o.had_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breach(b: bool, had_data: bool) -> ConditionOutcome {
        ConditionOutcome {
            breaching: b,
            had_data,
            value: if had_data { Some(1.0) } else { None },
        }
    }

    #[test]
    fn and_requires_all_or_requires_any() {
        let both = [breach(true, true), breach(true, true)];
        let one = [breach(true, true), breach(false, true)];
        assert!(combine(&both, Combinator::And));
        assert!(!combine(&one, Combinator::And));
        assert!(combine(&one, Combinator::Or));
        assert!(!combine(&[breach(false, true)], Combinator::Or));
    }

    #[test]
    fn empty_conditions_never_breach() {
        assert!(!combine(&[], Combinator::And));
        assert!(!combine(&[], Combinator::Or));
    }

    #[test]
    fn no_data_detected_if_any_condition_empty() {
        assert!(any_no_data(&[breach(true, true), breach(false, false)]));
        assert!(!any_no_data(&[breach(true, true), breach(false, true)]));
    }

    #[test]
    fn combinator_round_trips_and_defaults_to_and() {
        assert_eq!(Combinator::parse("or"), Combinator::Or);
        assert_eq!(Combinator::parse("and"), Combinator::And);
        assert_eq!(Combinator::parse("???"), Combinator::And);
        assert_eq!(Combinator::And.as_str(), "and");
    }
}
