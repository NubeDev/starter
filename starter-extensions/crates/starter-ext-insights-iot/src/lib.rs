//! # starter-ext-insights-iot
//!
//! Phase 1 IoT rule pack — ships three reusable [`Rule`] impls and
//! three namespaced quality flags so the canonical IoT row in
//! `DOCS/Insights/SCOPE.md` runs end-to-end against the
//! `starter-insights` registries.
//!
//! Rules (assertion / point-in-time):
//! - `iot.device.online@1` — verdict on whether `last_seen_secs_ago
//!   <= threshold_secs`. Input `params.last_seen_secs_ago: i64`,
//!   `params.threshold_secs: i64`.
//! - `iot.sensor.has-recent-data@1` — fires `Warn` when no sample
//!   has landed within `window_secs`. Input
//!   `params.last_sample_secs_ago: i64`, `params.window_secs: i64`.
//! - `iot.sensor.in-range@1` — fires `Critical` when `value` is
//!   outside `[min, max]`. Input `params.value: f64`,
//!   `params.min: f64`, `params.max: f64`.
//!
//! Quality flags (namespaced under `iot.quality`):
//! - `iot.quality.clock-skew@1`
//! - `iot.quality.sensor-swap@1`
//! - `iot.quality.late-arrival@1`
//!
//! The pack exposes two free functions, [`register_rules`] and
//! [`register_quality_flags`], invoked by the host at boot. This
//! mirrors the `contributes.tools`/`register()` extension surface
//! per R-ins-1 — the host crate depends on `starter-insights` and
//! does the wiring; the pack itself depends on `starter-spi` only.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::insights::{
    Coverage, QualityFlag, QualityFlagId, QualityFlagSeverity, Rule, RuleId, RuleInput,
    RuleOutput, RuleSchema, Tags, Verdict, OUT_OF_RANGE, STUCK,
};

/// Static rule tags — every IoT rule carries `domain:iot` and
/// `kind:assertion` per R-ins-8.
fn iot_tags() -> Tags {
    Tags::empty()
        .with_value("domain", "iot")
        .with_value("kind", "assertion")
}

// ----------------------------------------------------------------
// iot.device.online@1
// ----------------------------------------------------------------

/// `iot.device.online@1` — verdict on whether a device's last
/// heartbeat lies within the configured threshold.
pub struct DeviceOnline {
    schema: RuleSchema,
}

impl DeviceOnline {
    /// Construct a default-tagged [`DeviceOnline`].
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("iot", "device.online", 1))
                .with_tags(iot_tags()),
        }
    }
}

impl Default for DeviceOnline {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for DeviceOnline {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let last_seen = read_i64(&input, "last_seen_secs_ago");
        let threshold = read_i64(&input, "threshold_secs");
        let (last_seen, threshold) = match (last_seen, threshold) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                return RuleOutput::Assertion(make_error(
                    &self.schema.id,
                    "iot.device.online: `last_seen_secs_ago` and `threshold_secs` required",
                ));
            }
        };
        let now = Utc::now();
        let verdict = if last_seen <= threshold {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("device online (last seen {last_seen}s ago, threshold {threshold}s)"),
            )
        } else {
            Verdict::critical(
                self.schema.id.clone(),
                now,
                format!("device offline (last seen {last_seen}s ago > threshold {threshold}s)"),
            )
        };
        RuleOutput::Assertion(verdict)
    }
}

// ----------------------------------------------------------------
// iot.sensor.has-recent-data@1
// ----------------------------------------------------------------

/// `iot.sensor.has-recent-data@1` — `Warn` when no sample has
/// arrived inside `window_secs`.
pub struct SensorHasRecentData {
    schema: RuleSchema,
}

impl SensorHasRecentData {
    /// Construct a default-tagged [`SensorHasRecentData`].
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("iot", "sensor.has-recent-data", 1))
                .with_tags(iot_tags()),
        }
    }
}

impl Default for SensorHasRecentData {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for SensorHasRecentData {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let last_sample = read_i64(&input, "last_sample_secs_ago");
        let window = read_i64(&input, "window_secs");
        let (last_sample, window) = match (last_sample, window) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                return RuleOutput::Assertion(make_error(
                    &self.schema.id,
                    "iot.sensor.has-recent-data: `last_sample_secs_ago` and `window_secs` required",
                ));
            }
        };
        let now = Utc::now();
        let verdict = if last_sample <= window {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("sensor data fresh (last sample {last_sample}s ago)"),
            )
        } else {
            let mut cov = Coverage::full_point();
            cov.quality_flags.push(QualityFlag::new(
                QualityFlagId::new("starter.quality", "gap", 1),
                QualityFlagSeverity::Warn,
            ));
            Verdict::warn(
                self.schema.id.clone(),
                now,
                format!(
                    "sensor data stale (last sample {last_sample}s ago > window {window}s)"
                ),
            )
            .with_coverage(cov)
        };
        RuleOutput::Assertion(verdict)
    }
}

// ----------------------------------------------------------------
// iot.sensor.in-range@1
// ----------------------------------------------------------------

/// `iot.sensor.in-range@1` — `Critical` when `value` is outside
/// `[min, max]`.
pub struct SensorInRange {
    schema: RuleSchema,
}

impl SensorInRange {
    /// Construct a default-tagged [`SensorInRange`].
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("iot", "sensor.in-range", 1))
                .with_tags(iot_tags()),
        }
    }
}

impl Default for SensorInRange {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for SensorInRange {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let v = read_f64(&input, "value");
        let lo = read_f64(&input, "min");
        let hi = read_f64(&input, "max");
        let (v, lo, hi) = match (v, lo, hi) {
            (Some(a), Some(b), Some(c)) => (a, b, c),
            _ => {
                return RuleOutput::Assertion(make_error(
                    &self.schema.id,
                    "iot.sensor.in-range: `value`, `min`, `max` required",
                ));
            }
        };
        let now = Utc::now();
        let verdict = if v < lo || v > hi {
            let mut cov = Coverage::full_point();
            cov.quality_flags.push(QualityFlag::new(
                QualityFlagId::new("starter.quality", "out-of-range", 1),
                QualityFlagSeverity::Critical,
            ));
            Verdict::critical(
                self.schema.id.clone(),
                now,
                format!("sensor value {v} outside [{lo}, {hi}]"),
            )
            .with_coverage(cov)
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("sensor value {v} within [{lo}, {hi}]"),
            )
        };
        RuleOutput::Assertion(verdict)
    }
}

// ----------------------------------------------------------------
// Registration seams
// ----------------------------------------------------------------

/// Boxed `Rule` impls for the host's `RuleRegistry::register()`
/// loop. The host crate (which depends on `starter-insights`)
/// wires these into the registry at boot.
pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(DeviceOnline::new()),
        Arc::new(SensorHasRecentData::new()),
        Arc::new(SensorInRange::new()),
    ]
}

/// `iot.quality.*` flag descriptors. Each entry is
/// `(QualityFlagId, description, remediation)`.
pub fn quality_flags() -> Vec<(QualityFlagId, &'static str, &'static str)> {
    vec![
        (
            QualityFlagId::new("iot.quality", "clock-skew", 1),
            "sample timestamps drifting from wall-clock",
            "check the device's NTP / RTC; samples beyond the skew tolerance may belong to a different window",
        ),
        (
            QualityFlagId::new("iot.quality", "sensor-swap", 1),
            "calibration changed under the same device_id",
            "verify recent maintenance; treat samples either side of the swap as different series",
        ),
        (
            QualityFlagId::new("iot.quality", "late-arrival", 1),
            "sample arrived after its window closed",
            "increase the align reorder buffer or accept the late sample as a retroactive correction",
        ),
    ]
}

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

fn read_i64(input: &RuleInput, key: &str) -> Option<i64> {
    input.param(key).and_then(|v| v.as_i64())
}

fn read_f64(input: &RuleInput, key: &str) -> Option<f64> {
    input.param(key).and_then(|v| v.as_f64())
}

fn make_error(id: &RuleId, summary: &str) -> Verdict {
    let mut cov = Coverage::full_point();
    cov.quality_flags
        .push(starter_spi::insights::rule_error_flag(
            starter_spi::insights::RuleErrorKind::InputMissing,
        ));
    Verdict::error(id.clone(), Utc::now(), summary).with_coverage(cov)
}

/// Re-export of the three built-in quality flag string constants
/// the IoT rules emit alongside their `iot.quality.*` flags
/// (R-ins-11).
pub mod builtin_flag_ids {
    /// `starter.quality.stuck` constant — used by IoT analyzers
    /// that detect identical consecutive samples.
    pub const STUCK: &str = super::STUCK;
    /// `starter.quality.out-of-range`.
    pub const OUT_OF_RANGE: &str = super::OUT_OF_RANGE;
}
