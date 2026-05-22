//! D5 — retroactive correction seam.
//!
//! The engine — never the rule body — attaches the
//! [`starter_spi::insights::retroactive_correction_flag`] to verdicts
//! produced from windows that overlap a mutated input watermark.
//!
//! This module ships the watermark registry + the
//! [`attach_retroactive_flag`] helper the engine calls on every
//! emitted verdict.  Sources that don't expose a `mutated_at`
//! watermark are treated as immutable per the SCOPE doc (D5).
//!
//! Pairing with rollup invalidation lives in
//! [`crate::rollups::RollupEngine::on_input_mutation`] — when an input
//! mutates, the caller (a) registers the mutation here so future
//! verdicts pick up the flag, and (b) enqueues per-window
//! invalidations so the scheduled rollup tick re-aggregates the
//! affected windows.

use chrono::{DateTime, Utc};
use starter_spi::insights::{retroactive_correction_flag, RuleSchema, Verdict};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// A registry of input-source mutation watermarks (D5).
///
/// Keyed by an opaque source identifier (the engine wires its own
/// source-id convention — typically `"source.<name>@<rev>"`).  Each
/// entry records the wall-clock instant the source was last mutated;
/// verdicts whose `window` overlaps `[mutated_at, +inf)` are flagged
/// as retroactive.
///
/// The registry is cheap to clone (it's an `Arc`-wrapped `RwLock`),
/// so the engine can hand the same handle to every rule node and the
/// rollup drainer without coordinating ownership.
#[derive(Clone, Default)]
pub struct MutationWatermarks {
    inner: Arc<RwLock<BTreeMap<String, DateTime<Utc>>>>,
}

impl MutationWatermarks {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record (or update) the mutation watermark for a source.
    pub fn record(&self, source_id: impl Into<String>, mutated_at: DateTime<Utc>) {
        let mut g = self.inner.write().expect("MutationWatermarks poisoned");
        g.insert(source_id.into(), mutated_at);
    }

    /// Read a source's most-recent mutation watermark.
    pub fn get(&self, source_id: &str) -> Option<DateTime<Utc>> {
        self.inner
            .read()
            .expect("MutationWatermarks poisoned")
            .get(source_id)
            .copied()
    }

    /// Whether *any* registered watermark falls within
    /// `[window_start, window_end)` — used by
    /// [`attach_retroactive_flag`] as the "this verdict's window
    /// overlaps a mutated input" predicate.
    pub fn any_within(
        &self,
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> bool {
        let g = self.inner.read().expect("MutationWatermarks poisoned");
        // A watermark at exactly `window_end` is treated as outside —
        // windows are `[start, end)`.
        g.values().any(|w| *w >= window_start && *w < window_end)
    }

    /// Drop a source's watermark — useful for tests, and for the
    /// engine when a source revision is retired.
    pub fn clear(&self, source_id: &str) {
        let mut g = self.inner.write().expect("MutationWatermarks poisoned");
        g.remove(source_id);
    }
}

/// Engine seam: attach the
/// `starter.quality.retroactive-correction@1` flag to a verdict iff
/// the rule declares `retroactive: true` AND any input source's
/// mutation watermark falls inside the verdict's window.
///
/// Returns `true` when a flag was attached. Idempotent — if the
/// flag is already present, the function is a no-op.  This is the
/// **only** way to attach the flag; rule bodies that touch
/// `coverage.quality_flags` directly are caught by the determinism
/// smoke (R-ins-2).
pub fn attach_retroactive_flag(
    verdict: &mut Verdict,
    schema: &RuleSchema,
    watermarks: &MutationWatermarks,
) -> bool {
    if !schema.retroactive {
        return false;
    }
    if !watermarks.any_within(verdict.window.start, verdict.window.end) {
        return false;
    }
    let already_present = verdict
        .coverage
        .quality_flags
        .iter()
        .any(|f| f.id.namespace == "starter.quality" && f.id.name == "retroactive-correction");
    if already_present {
        return false;
    }
    verdict
        .coverage
        .quality_flags
        .push(retroactive_correction_flag());
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use starter_spi::insights::{RuleId, RuleSchema, Severity, Verdict, Window};

    fn schema(retro: bool) -> RuleSchema {
        let s = RuleSchema::derivation(RuleId::new("t", "r", 1));
        if retro {
            s.retroactive()
        } else {
            s
        }
    }

    fn vw(start: DateTime<Utc>, end: DateTime<Utc>) -> Verdict {
        Verdict::new(RuleId::new("t", "r", 1), start, Severity::Healthy, "ok")
            .with_window(Window::new(start, end))
    }

    #[test]
    fn no_flag_when_rule_is_not_retroactive() {
        let wm = MutationWatermarks::new();
        wm.record(
            "src",
            Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap(),
        );
        let mut v = vw(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        );
        let attached = attach_retroactive_flag(&mut v, &schema(false), &wm);
        assert!(!attached);
        assert!(v.coverage.quality_flags.is_empty());
    }

    #[test]
    fn no_flag_when_no_watermark_inside_window() {
        let wm = MutationWatermarks::new();
        wm.record(
            "src",
            Utc.with_ymd_and_hms(2024, 2, 1, 12, 0, 0).unwrap(),
        );
        let mut v = vw(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        );
        let attached = attach_retroactive_flag(&mut v, &schema(true), &wm);
        assert!(!attached);
    }

    #[test]
    fn flag_attached_when_retroactive_and_window_overlaps() {
        let wm = MutationWatermarks::new();
        wm.record(
            "src",
            Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap(),
        );
        let mut v = vw(
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        );
        let attached = attach_retroactive_flag(&mut v, &schema(true), &wm);
        assert!(attached);
        assert_eq!(v.coverage.quality_flags.len(), 1);
        assert_eq!(
            v.coverage.quality_flags[0].id.name,
            "retroactive-correction"
        );

        // Idempotent.
        let again = attach_retroactive_flag(&mut v, &schema(true), &wm);
        assert!(!again);
        assert_eq!(v.coverage.quality_flags.len(), 1);
    }
}
