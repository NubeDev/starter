//! The `AnomalyRule` trait and the data shapes it operates on.
//!
//! Kept deliberately tiny: a [`Reading`] is just the columns we
//! actually read from L1's `samples` table, a [`WindowSlice`] is a
//! slice of preceding readings, and an [`AnomalyRule`] produces a
//! [`RuleOutcome`]. No DB types, no async, no error type — rules
//! are pure functions over already-loaded rows.

use serde::{Deserialize, Serialize};

/// One row from `samples`, projected to the columns the cleaner
/// actually inspects.
///
/// `value` is `Option<f64>` because `samples.value_num` is nullable;
/// `None` represents an explicit null (distinct from `Some(NaN)`
/// which is the synth-injected "broken sensor" case). Rules that
/// don't care about strings/booleans just ignore them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reading {
    /// `samples.tenant_id` — the cleaner partitions by this; rules
    /// never see two tenants' rows in one window.
    pub tenant_id: String,
    /// `samples.entity_id` — same partition guarantee as tenant.
    pub entity_id: String,
    /// `samples.ts` as epoch milliseconds. Stored as TIMESTAMPTZ
    /// in Postgres; the cleaner converts at fetch time so rules
    /// can do simple numeric arithmetic on time deltas.
    pub ts_ms: i64,
    /// `samples.value_num`. `None` = SQL NULL.
    pub value: Option<f64>,
    /// Source quality from L1 (`samples.quality` SMALLINT).
    /// `0` = ok, `1` = suspect, `2` = missing — matches
    /// `ReadingQuality` in `rubix-spi`. Rules may consult this if
    /// they care about upstream-tagged rows.
    pub source_quality: i16,
}

/// Window slice handed to [`AnomalyRule::apply`].
///
/// Always **chronologically ascending** and **same `(tenant, entity)`**
/// as the focal row. Empty for the first row in a window; rules
/// that need history MUST handle that case.
#[derive(Debug, Clone, Copy)]
pub struct WindowSlice<'a> {
    /// Preceding readings in chronological order, oldest first.
    pub history: &'a [Reading],
}

impl<'a> WindowSlice<'a> {
    /// New window slice over `history`. The caller is responsible
    /// for the chronological + same-entity invariant.
    pub fn new(history: &'a [Reading]) -> Self {
        Self { history }
    }

    /// The most recent reading in `history`, if any. Convenient
    /// shortcut for rules that only look at the previous row.
    pub fn last(&self) -> Option<&Reading> {
        self.history.last()
    }
}

/// Quality verdict produced by a rule. The cleaner persists this
/// as TEXT in L2's `quality` column. Distinct from L1's numeric
/// `SMALLINT` because L2 carries richer detection state than L1's
/// `ok/suspect/missing` trio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityTag {
    /// Healthy reading; the cleaner only emits this when **every**
    /// registered rule returned [`RuleOutcome::Ok`].
    Ok,
    /// A spike, per the rule's threshold definition.
    Spike,
    /// Stuck reading: identical to recent history beyond the
    /// rule's `min_repeats` threshold.
    Stuck,
    /// Gap: detected by absence (window-level), not per-row. The
    /// cleaner tick emits Missing rows for buckets with no input;
    /// per-row rules typically don't produce this.
    Missing,
    /// `value.is_nan()` (synth-injected NaN, sensor wedged).
    Nan,
}

impl QualityTag {
    /// Render as the lowercase string the L2 `quality` column
    /// stores. Stable wire shape — operators query on this.
    pub fn as_str(self) -> &'static str {
        match self {
            QualityTag::Ok => "ok",
            QualityTag::Spike => "spike",
            QualityTag::Stuck => "stuck",
            QualityTag::Missing => "missing",
            QualityTag::Nan => "nan",
        }
    }
}

/// Per-row rule outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum RuleOutcome {
    /// Row passes this rule; the cleaner moves to the next rule
    /// (or emits with [`QualityTag::Ok`] if no rule flagged).
    Ok,
    /// Row is flagged with `quality`. Optional `note` becomes a
    /// JSONB entry in L2's `tags` column under the rule's id.
    Flag {
        /// The quality tag this rule assigns.
        quality: QualityTag,
        /// Free-form explanatory note. `None` ⇒ no tags entry.
        note: Option<String>,
    },
    /// Row should not appear in L2 at all (e.g. detected as a
    /// pure transport artifact). Use sparingly — Flag preserves
    /// the row for audit; Drop discards it.
    Drop,
}

/// Per-row anomaly detector.
///
/// `apply` is **sync, infallible** by design. Rules that need
/// external lookups (DB, network) belong in #3b as extension
/// rules dispatched through a tool call — the in-process trait
/// stays a pure function for cleaner-loop throughput.
pub trait AnomalyRule: Send + Sync + std::fmt::Debug {
    /// Stable rule id (reverse-DNS for extensions, simple snake
    /// for builtins). Logged on every fire so operators can
    /// trace which rule produced a tag.
    fn id(&self) -> &'static str;

    /// Inspect `row` in the context of `window` and decide.
    fn apply(&self, row: &Reading, window: WindowSlice<'_>) -> RuleOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_tag_strings_are_stable() {
        assert_eq!(QualityTag::Ok.as_str(), "ok");
        assert_eq!(QualityTag::Spike.as_str(), "spike");
        assert_eq!(QualityTag::Stuck.as_str(), "stuck");
        assert_eq!(QualityTag::Missing.as_str(), "missing");
        assert_eq!(QualityTag::Nan.as_str(), "nan");
    }

    #[test]
    fn window_slice_last_returns_most_recent() {
        let history = vec![
            Reading {
                tenant_id: "t".into(),
                entity_id: "e".into(),
                ts_ms: 1,
                value: Some(1.0),
                source_quality: 0,
            },
            Reading {
                tenant_id: "t".into(),
                entity_id: "e".into(),
                ts_ms: 2,
                value: Some(2.0),
                source_quality: 0,
            },
        ];
        let w = WindowSlice::new(&history);
        assert_eq!(w.last().unwrap().ts_ms, 2);
    }

    #[test]
    fn window_slice_empty_last_is_none() {
        let history: Vec<Reading> = vec![];
        let w = WindowSlice::new(&history);
        assert!(w.last().is_none());
    }
}
