//! [`Coverage`] — first-class confidence + quality flag bag carried
//! on every `Verdict` and `Dataset` (Insights SCOPE R-ins-6).

use serde::{Deserialize, Serialize};

use super::quality::QualityFlag;
use super::rule::RuleId;

/// Sample-counted, source-anchored confidence. Immutable once set
/// (at the source node, or at `align`); downstream copies through
/// unchanged. A rule body that touches `raw` is a bug.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RawCoverage {
    /// Samples the source expected in the window.
    pub samples_expected: u64,
    /// Original samples actually present (never synthetics).
    pub samples_present: u64,
    /// Source-anchored confidence in `[0.0, 1.0]`. Set at the
    /// source / align node; NEVER mutated downstream.
    pub confidence: f32,
}

impl RawCoverage {
    /// Construct a [`RawCoverage`]. Phase 1 IoT rules build this on
    /// every verdict; future windowing nodes own it.
    pub fn new(samples_expected: u64, samples_present: u64, confidence: f32) -> Self {
        Self {
            samples_expected,
            samples_present,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// Point-in-time helper — one expected sample, one present,
    /// full confidence.
    pub fn full_point() -> Self {
        Self::new(1, 1, 1.0)
    }
}

/// Effective confidence — `raw.confidence` discounted by every
/// derivation rule's declared `confidence_penalty`. The engine
/// mutates this; rule bodies must not (caught by the determinism
/// smoke). `gate` reads this, never `raw`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EffectiveCoverage {
    /// Derived confidence in `[0.0, 1.0]`. Initialised to
    /// `raw.confidence`; multiplied by each derivation penalty.
    pub confidence: f32,
    /// Audit chain — `(rule_id, penalty)` for every derivation
    /// that touched this dataset. Grows monotonically; never
    /// shrinks.
    pub penalty_chain: Vec<(RuleId, f32)>,
}

impl EffectiveCoverage {
    /// Construct an [`EffectiveCoverage`] starting from a raw
    /// confidence value. The penalty chain starts empty.
    pub fn from_raw(raw: &RawCoverage) -> Self {
        Self {
            confidence: raw.confidence,
            penalty_chain: Vec::new(),
        }
    }

    /// Construct from confidence + penalty chain. Use this instead
    /// of struct-expression initialisation; the struct is
    /// `#[non_exhaustive]`.
    pub fn from_parts(confidence: f32, penalty_chain: Vec<(RuleId, f32)>) -> Self {
        Self {
            confidence: confidence.clamp(0.0, 1.0),
            penalty_chain,
        }
    }
}

/// Coverage bundle on every `Verdict` / `Dataset` (R-ins-6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Coverage {
    /// Source-anchored, immutable downstream.
    pub raw: RawCoverage,
    /// Mutated only by the engine across derivations.
    pub effective: EffectiveCoverage,
    /// Quality flags (R-ins-11) attached at the source, at
    /// `align`, by derivation rules, or by `verdict.join`.
    pub quality_flags: Vec<QualityFlag>,
}

impl Coverage {
    /// Construct a [`Coverage`] from a [`RawCoverage`]; effective
    /// starts equal to raw, quality flags empty.
    pub fn from_raw(raw: RawCoverage) -> Self {
        let effective = EffectiveCoverage::from_raw(&raw);
        Self {
            raw,
            effective,
            quality_flags: Vec::new(),
        }
    }

    /// "Full point-in-time" helper — full confidence, no flags.
    /// Phase 1 IoT happy-path uses this on the latest sample.
    pub fn full_point() -> Self {
        Self::from_raw(RawCoverage::full_point())
    }

    /// Push a quality flag onto the coverage bundle.
    pub fn with_flag(mut self, flag: QualityFlag) -> Self {
        self.quality_flags.push(flag);
        self
    }

    /// Full constructor — explicit raw + effective + flags. The
    /// struct is `#[non_exhaustive]`; external crates use this
    /// instead of struct-expression initialisation.
    pub fn from_parts(
        raw: RawCoverage,
        effective: EffectiveCoverage,
        quality_flags: Vec<QualityFlag>,
    ) -> Self {
        Self {
            raw,
            effective,
            quality_flags,
        }
    }
}
