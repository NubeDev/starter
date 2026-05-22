//! # starter-ext-insights-hvac
//!
//! Phase 3 HVAC rule pack — three assertion rules per the SCOPE
//! use-case sanity-check (line 1178):
//!
//! - `hvac.pmv.comfort@1` — fires `Warn` when PMV drifts outside
//!   `[-0.5, +0.5]` (ASHRAE 55 comfort band) for the window.
//! - `hvac.setpoint.drift@1` — fires `Warn` when measured °C
//!   strays from setpoint by more than `tolerance_c` for longer
//!   than `dwell_minutes`.
//! - `hvac.short-cycle@1` — fires `Critical` when an AHU /
//!   compressor cycle count over `window_minutes` exceeds
//!   `max_cycles`.
//!
//! Quality flags:
//! - `hvac.quality.sensor-noise@1`
//! - `hvac.quality.bms-clock-skew@1`
//!
//! Rule packs depend on `starter-spi` only (D1).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::insights::{
    Coverage, QualityFlag, QualityFlagId, QualityFlagSeverity, Rule, RuleId, RuleInput,
    RuleOutput, RuleSchema, Severity, Tags, Verdict,
};

fn hvac_tags(name: &str) -> Tags {
    Tags::empty()
        .with_value("domain", "hvac")
        .with_value("kind", "assertion")
        .with_value("starter.rule.subkind", name)
}

// --------------------------------------------------------------------
// hvac.pmv.comfort@1
// --------------------------------------------------------------------

/// `hvac.pmv.comfort@1` — Predicted-Mean-Vote comfort band check.
///
/// Inputs (`params`):
/// - `pmv`: f64 — Fanger PMV value for the window (caller provides;
///   typically computed by a derivation rule upstream).
/// - `band_low`: f64 — defaults to `-0.5`.
/// - `band_high`: f64 — defaults to `+0.5`.
pub struct PmvComfort {
    schema: RuleSchema,
}

impl PmvComfort {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("hvac", "pmv.comfort", 1))
                .with_tags(hvac_tags("pmv-comfort")),
        }
    }
}

impl Default for PmvComfort {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for PmvComfort {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }
    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let pmv = match input.param("pmv").and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => return RuleOutput::Assertion(missing(&self.schema.id, "pmv")),
        };
        let lo = input
            .param("band_low")
            .and_then(|v| v.as_f64())
            .unwrap_or(-0.5);
        let hi = input
            .param("band_high")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        let now = Utc::now();
        let v = if pmv < lo || pmv > hi {
            Verdict::warn(
                self.schema.id.clone(),
                now,
                format!("PMV {pmv:.2} outside [{lo:.2}, {hi:.2}]"),
            )
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("PMV {pmv:.2} within [{lo:.2}, {hi:.2}]"),
            )
        };
        RuleOutput::Assertion(v)
    }
}

// --------------------------------------------------------------------
// hvac.setpoint.drift@1
// --------------------------------------------------------------------

/// `hvac.setpoint.drift@1` — drift-from-setpoint check.
///
/// Inputs (`params`):
/// - `measured_c`: f64
/// - `setpoint_c`: f64
/// - `tolerance_c`: f64 (defaults `1.0`)
/// - `dwell_minutes`: f64 (defaults `15.0`) — informational only at
///   this layer; the upstream window/derivation rule is what makes
///   "dwell" meaningful.
pub struct SetpointDrift {
    schema: RuleSchema,
}

impl SetpointDrift {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("hvac", "setpoint.drift", 1))
                .with_tags(hvac_tags("setpoint-drift")),
        }
    }
}

impl Default for SetpointDrift {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for SetpointDrift {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }
    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let m = input.param("measured_c").and_then(|v| v.as_f64());
        let s = input.param("setpoint_c").and_then(|v| v.as_f64());
        let (m, s) = match (m, s) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                return RuleOutput::Assertion(missing(
                    &self.schema.id,
                    "measured_c+setpoint_c",
                ))
            }
        };
        let tol = input
            .param("tolerance_c")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);
        let now = Utc::now();
        let drift = (m - s).abs();
        let v = if drift > tol {
            Verdict::warn(
                self.schema.id.clone(),
                now,
                format!("setpoint drift {drift:.2} °C > tolerance {tol:.2} °C"),
            )
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("setpoint drift {drift:.2} °C within tolerance"),
            )
        };
        RuleOutput::Assertion(v)
    }
}

// --------------------------------------------------------------------
// hvac.short-cycle@1
// --------------------------------------------------------------------

/// `hvac.short-cycle@1` — compressor / AHU cycling check.
///
/// Inputs (`params`):
/// - `cycles`: i64 — cycle count over the upstream window.
/// - `max_cycles`: i64 — defaults `6` (≈ 1 cycle / 10 min in a
///   60-min window).
pub struct ShortCycle {
    schema: RuleSchema,
}

impl ShortCycle {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("hvac", "short-cycle", 1))
                .with_tags(hvac_tags("short-cycle")),
        }
    }
}

impl Default for ShortCycle {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for ShortCycle {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }
    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let c = match input.param("cycles").and_then(|v| v.as_i64()) {
            Some(v) => v,
            None => return RuleOutput::Assertion(missing(&self.schema.id, "cycles")),
        };
        let max = input
            .param("max_cycles")
            .and_then(|v| v.as_i64())
            .unwrap_or(6);
        let now = Utc::now();
        let v = if c > max {
            Verdict::critical(
                self.schema.id.clone(),
                now,
                format!("short-cycling: {c} cycles > max {max}"),
            )
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("cycling within budget: {c} <= {max}"),
            )
        };
        RuleOutput::Assertion(v)
    }
}

// --------------------------------------------------------------------
// Registration seams.
// --------------------------------------------------------------------

fn missing(id: &RuleId, what: &str) -> Verdict {
    let mut cov = Coverage::full_point();
    cov.quality_flags
        .push(starter_spi::insights::rule_error_flag(
            starter_spi::insights::RuleErrorKind::InputMissing,
        ));
    let _ = Severity::Error;
    Verdict::error(
        id.clone(),
        Utc::now(),
        format!("{id}: missing required input `{what}`"),
    )
    .with_coverage(cov)
}

/// Boxed `Rule` impls for the host's `RuleRegistry::register()` loop.
pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(PmvComfort::new()),
        Arc::new(SetpointDrift::new()),
        Arc::new(ShortCycle::new()),
    ]
}

/// `hvac.quality.*` flag descriptors.
pub fn quality_flags() -> Vec<(QualityFlagId, &'static str, &'static str)> {
    vec![
        (
            QualityFlagId::new("hvac.quality", "sensor-noise", 1),
            "temperature / humidity sensor reading is noisy beyond the rule's tolerance",
            "investigate the sensor wiring or chain a denoising derivation rule upstream",
        ),
        (
            QualityFlagId::new("hvac.quality", "bms-clock-skew", 1),
            "BMS event timestamps drift relative to wall clock — cycle counting may be off",
            "resync the BMS NTP / time source; widen the short-cycle window until corrected",
        ),
    ]
}

/// Convenience: a `QualityFlag` value for the sensor-noise flag.
pub fn sensor_noise_flag() -> QualityFlag {
    QualityFlag::new(
        QualityFlagId::new("hvac.quality", "sensor-noise", 1),
        QualityFlagSeverity::Info,
    )
}
