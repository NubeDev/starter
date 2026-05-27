//! Builtin [`AnomalyRule`] implementations.
//!
//! These cover the four mess injectors `synth::mess` emits today
//! ([`crate::dataflow::mess`]): `nan`, `spike`, `stuck`,
//! `gap`/`missing`. NaN and stuck are detectable per-row;
//! spike is detectable with a one-row window (compare to last);
//! missing is window-level (detected by the cleaner's window
//! walker, not by a per-row rule) and therefore not included here.

use super::rule::{AnomalyRule, QualityTag, Reading, RuleOutcome, WindowSlice};

/// Flag rows whose `value` is `NaN`. Mirrors `synth::mess::nan_fires`.
///
/// `NaN` cannot be stored in Postgres `DOUBLE PRECISION` (the
/// existing ingest tool skips it; see
/// [`crate::warehouse::ingest`]). For L2 we keep the row but flag
/// it so dashboards can render "sensor wedged" without scanning L1.
#[derive(Debug, Default, Clone, Copy)]
pub struct NanRule;

impl AnomalyRule for NanRule {
    fn id(&self) -> &'static str {
        "builtin.nan"
    }

    fn apply(&self, row: &Reading, _window: WindowSlice<'_>) -> RuleOutcome {
        match row.value {
            Some(v) if v.is_nan() => RuleOutcome::Flag {
                quality: QualityTag::Nan,
                note: Some("value is NaN".into()),
            },
            _ => RuleOutcome::Ok,
        }
    }
}

/// Flag rows whose `value` is a large multiple of the previous
/// reading. Mirrors `synth::mess::spike_value` (×50 of last clean
/// step) with a more conservative default factor.
///
/// Detector: |value| ≥ `factor` × |last_value|. Rule does not fire
/// on the first row in a window (no `last` to compare to) — the
/// cleaner's window walker eventually provides one as it scans
/// forward.
///
/// `note` carries the actual ratio so an operator can see how
/// extreme the spike was without re-fetching history.
#[derive(Debug, Clone, Copy)]
pub struct SpikeRule {
    /// Multiplicative threshold against the previous reading.
    /// Default = 10.0 (conservative; synth emits ×50, real-world
    /// noise should not approach this).
    pub factor: f64,
}

impl Default for SpikeRule {
    fn default() -> Self {
        Self { factor: 10.0 }
    }
}

impl AnomalyRule for SpikeRule {
    fn id(&self) -> &'static str {
        "builtin.spike"
    }

    fn apply(&self, row: &Reading, window: WindowSlice<'_>) -> RuleOutcome {
        let Some(curr) = row.value else {
            return RuleOutcome::Ok;
        };
        if curr.is_nan() {
            // Let `NanRule` (registered first by the registry's
            // default) own NaN; spike compares numerically so a
            // NaN here would produce false negatives.
            return RuleOutcome::Ok;
        }
        let Some(last_row) = window.last() else {
            return RuleOutcome::Ok;
        };
        let Some(last) = last_row.value else {
            return RuleOutcome::Ok;
        };
        if last == 0.0 {
            // Avoid divide-by-zero. A jump from 0 to non-zero is
            // not a spike — it's a sensor coming online.
            return RuleOutcome::Ok;
        }
        let ratio = (curr / last).abs();
        if ratio >= self.factor {
            return RuleOutcome::Flag {
                quality: QualityTag::Spike,
                note: Some(format!("ratio={ratio:.1}× vs previous")),
            };
        }
        RuleOutcome::Ok
    }
}

/// Flag rows whose `value` repeats the last `min_repeats`
/// readings exactly. Mirrors `synth::mess::maybe_start_stuck`.
///
/// A stuck sensor pinned at a single value is one of the most
/// common real-world failure modes (battery dead, dirty contacts,
/// software defaulting to last-good). Detector counts consecutive
/// equal-value rows at the tail of the window and fires when the
/// count reaches `min_repeats`.
#[derive(Debug, Clone, Copy)]
pub struct StuckRule {
    /// Minimum number of preceding identical readings required to
    /// flag the current row. Default = 3 (matches the synth-side
    /// stuck stretch's 10-tick floor with margin to spare).
    pub min_repeats: usize,
}

impl Default for StuckRule {
    fn default() -> Self {
        Self { min_repeats: 3 }
    }
}

impl AnomalyRule for StuckRule {
    fn id(&self) -> &'static str {
        "builtin.stuck"
    }

    fn apply(&self, row: &Reading, window: WindowSlice<'_>) -> RuleOutcome {
        let Some(curr) = row.value else {
            return RuleOutcome::Ok;
        };
        if curr.is_nan() {
            return RuleOutcome::Ok;
        }
        let mut equal_tail: usize = 0;
        for r in window.history.iter().rev() {
            match r.value {
                Some(v) if !v.is_nan() && v == curr => equal_tail += 1,
                _ => break,
            }
            if equal_tail >= self.min_repeats {
                return RuleOutcome::Flag {
                    quality: QualityTag::Stuck,
                    note: Some(format!(
                        "value={curr} repeated for {equal_tail}+ readings"
                    )),
                };
            }
        }
        RuleOutcome::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(ts_ms: i64, value: Option<f64>) -> Reading {
        Reading {
            tenant_id: "t".into(),
            entity_id: "e".into(),
            ts_ms,
            value,
            source_quality: 0,
        }
    }

    // ----- NanRule -----

    #[test]
    fn nan_rule_flags_nan() {
        let row = r(10, Some(f64::NAN));
        let out = NanRule.apply(&row, WindowSlice::new(&[]));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Nan,
                ..
            }
        ));
    }

    #[test]
    fn nan_rule_passes_finite() {
        let row = r(10, Some(1.0));
        assert!(matches!(NanRule.apply(&row, WindowSlice::new(&[])), RuleOutcome::Ok));
    }

    #[test]
    fn nan_rule_passes_null() {
        let row = r(10, None);
        assert!(matches!(NanRule.apply(&row, WindowSlice::new(&[])), RuleOutcome::Ok));
    }

    // ----- SpikeRule -----

    #[test]
    fn spike_rule_no_history_is_ok() {
        let row = r(10, Some(1000.0));
        assert!(matches!(
            SpikeRule::default().apply(&row, WindowSlice::new(&[])),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn spike_rule_fires_at_threshold() {
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(150.0)); // 15× — over default factor of 10
        let out = SpikeRule::default().apply(&row, WindowSlice::new(&history));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Spike,
                ..
            }
        ));
    }

    #[test]
    fn spike_rule_passes_small_change() {
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(15.0)); // 1.5×
        assert!(matches!(
            SpikeRule::default().apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn spike_rule_handles_zero_baseline() {
        let history = vec![r(1, Some(0.0))];
        let row = r(2, Some(100.0));
        // Coming online from 0 isn't a spike.
        assert!(matches!(
            SpikeRule::default().apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn spike_rule_defers_nan_to_nan_rule() {
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(f64::NAN));
        assert!(matches!(
            SpikeRule::default().apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn spike_rule_custom_factor_lowers_bar() {
        let rule = SpikeRule { factor: 2.0 };
        let history = vec![r(1, Some(10.0))];
        let row = r(2, Some(25.0)); // 2.5× — over a 2.0× factor
        assert!(matches!(
            rule.apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Flag {
                quality: QualityTag::Spike,
                ..
            }
        ));
    }

    // ----- StuckRule -----

    #[test]
    fn stuck_rule_no_history_is_ok() {
        let row = r(10, Some(5.0));
        assert!(matches!(
            StuckRule::default().apply(&row, WindowSlice::new(&[])),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn stuck_rule_fires_after_three_repeats() {
        let history = vec![r(1, Some(5.0)), r(2, Some(5.0)), r(3, Some(5.0))];
        let row = r(4, Some(5.0));
        let out = StuckRule::default().apply(&row, WindowSlice::new(&history));
        assert!(matches!(
            out,
            RuleOutcome::Flag {
                quality: QualityTag::Stuck,
                ..
            }
        ));
    }

    #[test]
    fn stuck_rule_resets_on_break() {
        // Two-then-different breaks the streak; current row is
        // back to the stuck value but the streak only counts
        // contiguous equal-tail readings, so we need three more.
        let history = vec![r(1, Some(5.0)), r(2, Some(5.0)), r(3, Some(6.0))];
        let row = r(4, Some(5.0));
        assert!(matches!(
            StuckRule::default().apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Ok
        ));
    }

    #[test]
    fn stuck_rule_custom_min_repeats() {
        let rule = StuckRule { min_repeats: 1 };
        let history = vec![r(1, Some(5.0))];
        let row = r(2, Some(5.0));
        assert!(matches!(
            rule.apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Flag {
                quality: QualityTag::Stuck,
                ..
            }
        ));
    }

    #[test]
    fn stuck_rule_ignores_nulls_and_nan_in_history() {
        let history = vec![
            r(1, None),
            r(2, Some(f64::NAN)),
            r(3, Some(5.0)),
        ];
        let row = r(4, Some(5.0));
        // The streak is broken by the None / NaN entries — only
        // the last `Some(5.0)` is in the equal-tail.
        assert!(matches!(
            StuckRule::default().apply(&row, WindowSlice::new(&history)),
            RuleOutcome::Ok
        ));
    }
}
