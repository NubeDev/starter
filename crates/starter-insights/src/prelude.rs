//! Public re-exports — the convenient surface for consumers and
//! rule packs that want to author a `Rule` without remembering the
//! module layout.

pub use crate::registry::{QualityFlagInfo, QualityFlagRegistry, RegistryError, RuleRegistry};
pub use starter_spi::insights::{
    join_all_inputs_errored_flag, rule_error_flag, Coverage, Dataset, DatasetRows, DatasetSchema,
    EffectiveCoverage, EvidenceRow, QualityFlag, QualityFlagId, QualityFlagSeverity, RawCoverage,
    Rule, RuleErrorKind, RuleId, RuleInput, RuleKind, RuleOutput, RuleSchema, Severity, TagValue,
    Tags, TimeZoneId, VecDatasetRows, Verdict, Window, GAP, JOIN_ALL_INPUTS_ERRORED, OUT_OF_RANGE,
    RULE_ERROR, STUCK, TAGS_TRUNCATED,
};
