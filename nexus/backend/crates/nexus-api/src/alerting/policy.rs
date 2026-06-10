//! The no-data and execution-error policies, as pure resolution.
//!
//! When a rule's query returns no rows, or errors, the rule has no natural
//! breaching boolean. The policy says what to do: treat it as `Ok`
//! (non-breaching — the historical default), `Alerting` (breaching, so the rule
//! fires), or `KeepLast` (carry the prior breaching decision forward, so a blip
//! in connectivity does not flap the rule). This module maps the policy to the
//! `breaching` the state machine consumes; the evaluator feeds it the prior
//! decision for `KeepLast`. No I/O here — the resolution is exhaustively tested.

/// What to do when a rule produces no data or its query errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Non-breaching — the historical default. A missing reading is not an alert.
    Ok,
    /// Breaching — fire on missing data (a stalled feed is itself the problem).
    Alerting,
    /// Carry the rule's last breaching decision forward, absorbing a transient gap.
    KeepLast,
}

impl Policy {
    /// Parse the stored string form. Defaults to `Ok`, the historical behaviour,
    /// so a missing or malformed policy never silently starts paging.
    pub fn parse(s: &str) -> Self {
        match s {
            "alerting" => Policy::Alerting,
            "keep_last" => Policy::KeepLast,
            _ => Policy::Ok,
        }
    }

    /// The stored string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Policy::Ok => "ok",
            Policy::Alerting => "alerting",
            Policy::KeepLast => "keep_last",
        }
    }
}

/// Resolve the `breaching` boolean for a no-data or error outcome.
///
/// - `policy` — the rule's no-data or error policy.
/// - `prior_firing` — whether the rule was firing before this evaluation (the
///   input to `KeepLast`; for a rule that has never fired this is `false`).
pub fn resolve(policy: Policy, prior_firing: bool) -> bool {
    match policy {
        Policy::Ok => false,
        Policy::Alerting => true,
        Policy::KeepLast => prior_firing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_is_non_breaching_regardless_of_prior() {
        assert!(!resolve(Policy::Ok, true));
        assert!(!resolve(Policy::Ok, false));
    }

    #[test]
    fn alerting_breaches_regardless_of_prior() {
        assert!(resolve(Policy::Alerting, false));
        assert!(resolve(Policy::Alerting, true));
    }

    #[test]
    fn keep_last_carries_the_prior_decision() {
        assert!(resolve(Policy::KeepLast, true));
        assert!(!resolve(Policy::KeepLast, false));
    }

    #[test]
    fn policy_round_trips_and_defaults_to_ok() {
        assert_eq!(Policy::parse("alerting"), Policy::Alerting);
        assert_eq!(Policy::parse("keep_last"), Policy::KeepLast);
        assert_eq!(Policy::parse("garbage"), Policy::Ok);
        assert_eq!(Policy::Alerting.as_str(), "alerting");
    }
}
