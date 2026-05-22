//! [`QualityFlag`] + the [`QualityFlagId`] registry shape
//! (Insights SCOPE R-ins-11).

use serde::{Deserialize, Serialize};
use std::fmt;

/// `(namespace, name, major)` registry identifier for a quality
/// flag. Same shape as `RuleId` — mechanically extensible by pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct QualityFlagId {
    /// Reverse-DNS namespace, e.g. `starter.quality`, `iot.quality`.
    pub namespace: String,
    /// Flag name, e.g. `gap`, `stuck`, `clock-skew`.
    pub name: String,
    /// Major version. Breaking changes bump this.
    pub major: u32,
}

impl QualityFlagId {
    /// Construct a [`QualityFlagId`].
    pub fn new(namespace: impl Into<String>, name: impl Into<String>, major: u32) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            major,
        }
    }
}

impl fmt::Display for QualityFlagId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}@{}", self.namespace, self.name, self.major)
    }
}

/// Severity attached to a quality flag (R-ins-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum QualityFlagSeverity {
    /// Informational — context for the operator, not actionable.
    Info,
    /// The data is suspect; downstream gates may suppress.
    Warn,
    /// The data should not be trusted.
    Critical,
}

/// An emitted quality flag. Carries a bounded `detail` for evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct QualityFlag {
    /// The flag's registry id.
    pub id: QualityFlagId,
    /// Severity at this emission.
    pub severity: QualityFlagSeverity,
    /// Optional bounded detail string — kept short for evidence rows.
    pub detail: Option<String>,
}

impl QualityFlag {
    /// Construct a quality flag.
    pub fn new(id: QualityFlagId, severity: QualityFlagSeverity) -> Self {
        Self {
            id,
            severity,
            detail: None,
        }
    }

    /// Attach a detail string. Builder shape.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Sub-kind tag carried on a `starter.quality.rule-error@1` flag's
/// `detail` (R-ins-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuleErrorKind {
    /// Rule body itself errored.
    BodyFailed,
    /// A required input slot was missing.
    InputMissing,
    /// Rule budget (Rhai operations, time) was exhausted.
    BudgetExhausted,
}

impl RuleErrorKind {
    /// Stable short string used in the flag's `detail` field.
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleErrorKind::BodyFailed => "body_failed",
            RuleErrorKind::InputMissing => "input_missing",
            RuleErrorKind::BudgetExhausted => "budget_exhausted",
        }
    }
}

// ----- Built-in flag ids (R-ins-11) -----

/// `starter.quality.gap@1` — samples missing in a window.
pub const GAP: &str = "starter.quality.gap";
/// `starter.quality.stuck@1` — N consecutive samples identical.
pub const STUCK: &str = "starter.quality.stuck";
/// `starter.quality.out-of-range@1` — value outside declared bounds.
pub const OUT_OF_RANGE: &str = "starter.quality.out-of-range";
/// `starter.quality.rule-error@1` — emitted on `Severity::Error`.
pub const RULE_ERROR: &str = "starter.quality.rule-error";
/// `starter.quality.join-all-inputs-errored@1` — all `verdict.join`
/// inputs errored.
pub const JOIN_ALL_INPUTS_ERRORED: &str = "starter.quality.join-all-inputs-errored";
/// `starter.quality.tags-truncated@1` — over the 32-tag cap.
pub const TAGS_TRUNCATED: &str = "starter.quality.tags-truncated";

/// Convenience: built-in `starter.quality.rule-error@1` flag.
pub fn rule_error_flag(kind: RuleErrorKind) -> QualityFlag {
    QualityFlag::new(
        QualityFlagId::new("starter.quality", "rule-error", 1),
        QualityFlagSeverity::Warn,
    )
    .with_detail(kind.as_str())
}

/// Convenience: built-in `starter.quality.join-all-inputs-errored@1`.
pub fn join_all_inputs_errored_flag() -> QualityFlag {
    QualityFlag::new(
        QualityFlagId::new("starter.quality", "join-all-inputs-errored", 1),
        QualityFlagSeverity::Critical,
    )
}
