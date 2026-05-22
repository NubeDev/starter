//! Onboarding-backfill cache-warming contract (Insights SCOPE
//! materialisation section).
//!
//! When a pipeline deploys for the first time (or a new tenant /
//! building / device is added), rollups and the derivation cache
//! are cold. The engine triggers a **bounded onboarding backfill**
//! capped at [`crate::backfill::BACKFILL_ROW_CAP`] (D3) over the
//! configured initial window (default 30 days, per-pipeline
//! overrideable). Rollups feed off the resulting verdict-log entries
//! on the next scheduled tick.
//!
//! This module owns the contract:
//!
//! - **`OnboardingPlan`** — the (rule, window) the engine intends
//!   to warm. Built by the host on pipeline-deploy events.
//! - **`run_onboarding_backfill`** — drives the backfill, applies
//!   the D3 cap, and reports back which plan rows were truncated
//!   so the tuner can propose a narrower window per the SCOPE doc.
//! - **Page loads never trigger backfills.** This invariant is
//!   enforced upstream of this module (the server's handler is a
//!   thin SELECT per the materialisation contract); this module is
//!   the seam the engine uses, never the seam the request handler
//!   uses.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use starter_spi::insights::{RuleId, Verdict};

use crate::backfill::{run_backfill, BackfillEvent};

/// Default initial onboarding window — 30 days, per SCOPE.
pub const DEFAULT_ONBOARDING_DAYS: u32 = 30;

/// A single rule / window the engine intends to warm on deploy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OnboardingPlan {
    /// The rule the engine is warming.
    pub rule_id: RuleId,
    /// Inclusive UTC start of the initial backfill window.
    pub window_start: DateTime<Utc>,
    /// Exclusive UTC end of the initial backfill window.
    pub window_end: DateTime<Utc>,
}

impl OnboardingPlan {
    /// Construct an [`OnboardingPlan`] over the default 30-day window
    /// ending at `now`.
    pub fn default_for(rule_id: RuleId, now: DateTime<Utc>) -> Self {
        Self {
            rule_id,
            window_start: now - chrono::Duration::days(DEFAULT_ONBOARDING_DAYS as i64),
            window_end: now,
        }
    }
}

/// Outcome of running an onboarding backfill.
///
/// Mirrors [`crate::backfill::BackfillOutcome`] with the plan that
/// drove it attached, so the tuner can correlate truncations back
/// to a specific deploy event.
#[derive(Debug, Clone)]
pub struct OnboardingOutcome {
    /// The plan that drove the backfill.
    pub plan: OnboardingPlan,
    /// Replayed verdicts (capped at [`BACKFILL_ROW_CAP`]; carry the
    /// `starter.quality.partial-onboarding@1` flag if truncated).
    pub verdicts: Vec<Verdict>,
    /// Backfill event.
    pub event: BackfillEvent,
}

/// Drive an onboarding backfill from an iterator of verdicts the
/// caller has already replayed against history.
///
/// The cap + `partial-onboarding` flag come from
/// [`crate::backfill::run_backfill`]; the only thing this wrapper
/// adds is attaching the [`OnboardingPlan`] to the outcome so the
/// tuner agent can route its proposal back to the right deploy
/// event.
///
/// **Cache warming and onboarding** is the SCOPE-doc section name —
/// rollups + derivation cache start cold, this is the bounded
/// machinery that warms them without stampeding the store.
pub fn run_onboarding_backfill(
    plan: OnboardingPlan,
    stream: impl IntoIterator<Item = Verdict>,
) -> OnboardingOutcome {
    let outcome = run_backfill(stream);
    OnboardingOutcome {
        plan,
        verdicts: outcome.verdicts,
        event: outcome.event,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backfill::BACKFILL_ROW_CAP;
    use chrono::TimeZone;
    use starter_spi::insights::{RuleId, Severity, Verdict};

    fn mk(i: u32, at: DateTime<Utc>) -> Verdict {
        Verdict::new(
            RuleId::new("t", "r", 1),
            at,
            Severity::Healthy,
            format!("v{i}"),
        )
    }

    #[test]
    fn default_plan_is_30_days_wide() {
        let now = Utc.with_ymd_and_hms(2024, 6, 1, 12, 0, 0).unwrap();
        let plan = OnboardingPlan::default_for(RuleId::new("t", "r", 1), now);
        assert_eq!((plan.window_end - plan.window_start).num_days(), 30);
    }

    #[test]
    fn truncation_marks_partial_onboarding() {
        let plan = OnboardingPlan::default_for(RuleId::new("t", "r", 1), Utc::now());
        let start = plan.window_start;
        let stream = (0..(BACKFILL_ROW_CAP + 5) as u32).map(|i| mk(i, start));
        let out = run_onboarding_backfill(plan, stream);
        assert!(matches!(out.event, BackfillEvent::Truncated { .. }));
        assert!(out.verdicts[0]
            .coverage
            .quality_flags
            .iter()
            .any(|f| f.id.name == "partial-onboarding"));
    }
}
