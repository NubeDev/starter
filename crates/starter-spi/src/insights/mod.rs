//! Insights capability — value types and the `Rule` trait.
//!
//! Per Insights SCOPE D1: `Rule`, `Verdict`, `Severity`, `Coverage`
//! (raw + effective), `Dataset` (+ `VecDatasetRows`), `RuleOutput`,
//! `Tags`, `QualityFlag`/`QualityFlagId`, `RuleId`, `Window`, and
//! `TimeZoneId` live in `starter-spi` so extension rule packs can
//! depend on this crate only (not `starter-insights`).
//!
//! Phase 1 (this stage) ships the type system + the `Rule` trait;
//! `RuleRegistry`, node bodies, sandboxes, persistence, and skill
//! bundles live in `starter-insights`.

mod coverage;
mod dataset;
mod quality;
mod rule;
mod tags;
mod time;
mod verdict;

pub use coverage::{Coverage, EffectiveCoverage, RawCoverage};
pub use dataset::{Dataset, DatasetRows, DatasetSchema, VecDatasetRows};
pub use quality::{
    join_all_inputs_errored_flag, partial_onboarding_flag, retroactive_correction_flag,
    rule_error_flag, QualityFlag, QualityFlagId, QualityFlagSeverity, RuleErrorKind, GAP,
    JOIN_ALL_INPUTS_ERRORED, OUT_OF_RANGE, PARTIAL_ONBOARDING, RETROACTIVE_CORRECTION, RULE_ERROR,
    STUCK, TAGS_TRUNCATED,
};
pub use rule::{Rule, RuleId, RuleInput, RuleKind, RuleOutput, RuleSchema};
pub use tags::{TagValue, Tags};
pub use time::{TimeZoneId, Window};
pub use verdict::{EvidenceRow, Severity, Verdict};
