//! [`Verdict`] + [`Severity`] + [`EvidenceRow`] — the only currency
//! between rule and action (Insights SCOPE R-ins-6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::coverage::Coverage;
use super::rule::RuleId;
use super::tags::Tags;
use super::time::{TimeZoneId, Window};

/// Verdict severity (R-ins-6). `Error` is **not** an exception —
/// it's a verdict variant. Rules emit it on body / input / budget
/// failure; downstream branches route on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Healthy — the rule's invariant held.
    Healthy,
    /// Informational — no action required.
    Info,
    /// Warning — investigate but non-blocking.
    Warn,
    /// Critical — gate-eligible, action-eligible.
    Critical,
    /// Rule could not produce an opinion. Carries a
    /// `starter.quality.rule-error@1` flag (R-ins-6). Never the
    /// result of `panic!` or `Err` — rules **convert** failures
    /// into this variant.
    Error,
}

impl Severity {
    /// Ordinal rank used for `verdict.join`'s `all` / `any`
    /// max-severity calculation. `Error` outranks everything
    /// because an error in one input cannot be silently squashed
    /// by a healthier sibling.
    pub fn rank(self) -> u8 {
        match self {
            Severity::Healthy => 0,
            Severity::Info => 1,
            Severity::Warn => 2,
            Severity::Critical => 3,
            Severity::Error => 4,
        }
    }
}

/// Single row of typed evidence supporting a [`Verdict`]. Phase 1
/// uses a free-form JSON value; later phases narrow this against
/// `DatasetSchema`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EvidenceRow {
    /// Free-form row payload.
    pub value: serde_json::Value,
}

impl EvidenceRow {
    /// Construct an evidence row from any JSON-serialisable value.
    pub fn new(value: serde_json::Value) -> Self {
        Self { value }
    }
}

/// The one currency between rule and action (R-ins-6).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Verdict {
    /// The rule (or synthetic-pipeline-rule for `verdict.join`)
    /// that emitted this verdict.
    pub rule_id: RuleId,
    /// UTC instant the verdict was emitted at.
    pub at: DateTime<Utc>,
    /// Time zone the analysis was performed against (DST-aware).
    pub tz: TimeZoneId,
    /// Time window the verdict covers. `Window::instant(at)` for
    /// point-in-time rules.
    pub window: Window,
    /// Verdict severity.
    pub severity: Severity,
    /// First-class coverage + quality flags.
    pub coverage: Coverage,
    /// Tag bag — rule tags ∪ pipeline-node tags (R-ins-8).
    pub tags: Tags,
    /// Short, machine + human readable summary.
    pub summary: String,
    /// Bounded typed evidence supporting the verdict.
    pub evidence: Vec<EvidenceRow>,
    /// Correlation id — joins to the engine run that emitted it.
    pub correlation_id: Option<Uuid>,
}

impl Verdict {
    /// Healthy-verdict builder. Phase 1 IoT rules use this for the
    /// happy path.
    pub fn healthy(rule_id: RuleId, at: DateTime<Utc>, summary: impl Into<String>) -> Self {
        Self::new(rule_id, at, Severity::Healthy, summary)
    }

    /// Flagged-verdict builder (`Severity::Warn`).
    pub fn warn(rule_id: RuleId, at: DateTime<Utc>, summary: impl Into<String>) -> Self {
        Self::new(rule_id, at, Severity::Warn, summary)
    }

    /// Critical-verdict builder.
    pub fn critical(rule_id: RuleId, at: DateTime<Utc>, summary: impl Into<String>) -> Self {
        Self::new(rule_id, at, Severity::Critical, summary)
    }

    /// Error-verdict builder (R-ins-6 "failure is a verdict"). The
    /// caller is responsible for attaching a
    /// `starter.quality.rule-error@1` flag to `coverage`; the
    /// [`super::rule_error_flag`] helper builds one.
    pub fn error(rule_id: RuleId, at: DateTime<Utc>, summary: impl Into<String>) -> Self {
        Self::new(rule_id, at, Severity::Error, summary)
    }

    /// Construct a [`Verdict`] with a degenerate point-in-time
    /// window, UTC timezone, full coverage, and no tags / evidence.
    /// Builder methods attach the rest.
    pub fn new(
        rule_id: RuleId,
        at: DateTime<Utc>,
        severity: Severity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            rule_id,
            at,
            tz: TimeZoneId::utc(),
            window: Window::instant(at),
            severity,
            coverage: Coverage::full_point(),
            tags: Tags::empty(),
            summary: summary.into(),
            evidence: Vec::new(),
            correlation_id: None,
        }
    }

    /// Override the timezone (builder).
    pub fn with_tz(mut self, tz: TimeZoneId) -> Self {
        self.tz = tz;
        self
    }

    /// Override the window (builder).
    pub fn with_window(mut self, window: Window) -> Self {
        self.window = window;
        self
    }

    /// Replace coverage (builder).
    pub fn with_coverage(mut self, coverage: Coverage) -> Self {
        self.coverage = coverage;
        self
    }

    /// Replace tags (builder).
    pub fn with_tags(mut self, tags: Tags) -> Self {
        self.tags = tags;
        self
    }

    /// Push evidence (builder).
    pub fn with_evidence(mut self, row: EvidenceRow) -> Self {
        self.evidence.push(row);
        self
    }

    /// Set the correlation id (builder).
    pub fn with_correlation(mut self, id: Uuid) -> Self {
        self.correlation_id = Some(id);
        self
    }
}
