//! # starter-ext-insights-finance
//!
//! Phase 4 finance rule pack — three assertion rules per the
//! DOCS/Insights/SCOPE.md "Relationship to existing crates" listing
//! (line 209) and the use-case sanity-check row 1174:
//!
//! - `finance.tx.z-score@1` — fires `Warn`/`Critical` when a
//!   transaction's amount is more than `threshold_sigma` standard
//!   deviations from the running mean of the supplied population.
//! - `finance.tx.isolation-forest-light@1` — fires `Warn` when a
//!   point falls in a sparsely-populated half-space of a tiny
//!   single-tree isolation forest built from the supplied
//!   population. Deterministic (seedable), no external deps —
//!   "light" by design.
//! - `finance.tx.duplicate@1` — fires `Critical` when the same
//!   `(account, amount, ts_bucket_secs)` triple appears more than
//!   once in the supplied window, modulo a configurable
//!   `bucket_secs` near-miss tolerance.
//!
//! Quality flags (per R-ins-11 + SCOPE line 1050):
//! - `finance.quality.duplicate-timestamp@1`
//! - `finance.quality.fx-rate-stale@1`
//!
//! Rule packs depend on `starter-spi` only (D1).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use starter_spi::insights::{
    Coverage, QualityFlag, QualityFlagId, QualityFlagSeverity, Rule, RuleId, RuleInput, RuleOutput,
    RuleSchema, Severity, Tags, Verdict,
};

fn finance_tags(name: &str) -> Tags {
    Tags::empty()
        .with_value("domain", "finance")
        .with_value("kind", "assertion")
        .with_value("starter.rule.subkind", name)
        // Finance pipelines are PII-sensitive — surface that on every
        // emitted verdict so routing and storage can react.
        .with_flag("pii")
}

// --------------------------------------------------------------------
// finance.tx.z-score@1
// --------------------------------------------------------------------

/// `finance.tx.z-score@1` — z-score outlier detection.
///
/// Inputs (`params`):
/// - `amount`: f64 — the transaction value being tested.
/// - `population`: JSON array of f64 — historical amounts for the
///   same account / merchant cohort. Caller is expected to have
///   filtered / windowed it upstream.
/// - `threshold_sigma`: f64 — defaults `3.0`.
/// - `critical_sigma`: f64 — defaults `5.0`. At or above this the
///   verdict escalates from `Warn` to `Critical`.
pub struct ZScore {
    schema: RuleSchema,
}

impl ZScore {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("finance", "tx.z-score", 1))
                .with_tags(finance_tags("z-score")),
        }
    }
}

impl Default for ZScore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for ZScore {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let amount = match input.param("amount").and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => return RuleOutput::Assertion(missing(&self.schema.id, "amount")),
        };
        let population: Vec<f64> = input
            .param("population")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_f64())
            .collect();
        if population.len() < 2 {
            // Not enough population to compute σ — coverage-degrade
            // rather than panic.
            return RuleOutput::Assertion(
                Verdict::healthy(
                    self.schema.id.clone(),
                    Utc::now(),
                    "z-score skipped: population < 2 samples".to_owned(),
                )
                .with_coverage(degraded_coverage()),
            );
        }
        let threshold = input
            .param("threshold_sigma")
            .and_then(|v| v.as_f64())
            .unwrap_or(3.0);
        let critical = input
            .param("critical_sigma")
            .and_then(|v| v.as_f64())
            .unwrap_or(5.0);

        let n = population.len() as f64;
        let mean = population.iter().sum::<f64>() / n;
        // Sample standard deviation (Bessel's correction); the
        // population is "historical samples", not the full universe.
        let var = population
            .iter()
            .map(|x| {
                let d = x - mean;
                d * d
            })
            .sum::<f64>()
            / (n - 1.0);
        let sd = var.sqrt();
        let now = Utc::now();
        if sd == 0.0 {
            // Degenerate distribution — only flag exact equals as
            // healthy; anything else as a Warn with a marker.
            let v = if amount == mean {
                Verdict::healthy(
                    self.schema.id.clone(),
                    now,
                    "z-score: σ=0, amount matches mean".to_owned(),
                )
            } else {
                Verdict::warn(
                    self.schema.id.clone(),
                    now,
                    format!("z-score: σ=0, amount {amount} != mean {mean}"),
                )
            };
            return RuleOutput::Assertion(v);
        }
        let z = (amount - mean) / sd;
        let abs_z = z.abs();
        let v = if abs_z >= critical {
            Verdict::critical(
                self.schema.id.clone(),
                now,
                format!("z-score |{z:.2}σ| >= critical {critical:.2}σ"),
            )
        } else if abs_z >= threshold {
            Verdict::warn(
                self.schema.id.clone(),
                now,
                format!("z-score |{z:.2}σ| >= threshold {threshold:.2}σ"),
            )
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("z-score |{z:.2}σ| within threshold {threshold:.2}σ"),
            )
        };
        RuleOutput::Assertion(v)
    }
}

// --------------------------------------------------------------------
// finance.tx.isolation-forest-light@1
// --------------------------------------------------------------------

/// `finance.tx.isolation-forest-light@1` — single-tree, deterministic
/// isolation-forest-style anomaly score.
///
/// Not a full isolation forest — "light" by design. Builds one
/// random-split binary partition from the supplied population using
/// a seeded PRNG, then measures the depth at which `value` is
/// isolated. Shallow isolation ⇒ outlier.
///
/// Inputs (`params`):
/// - `value`: f64
/// - `population`: JSON array of f64
/// - `max_depth`: i64 — defaults `8`. Splits beyond this are clamped.
/// - `threshold_depth`: i64 — defaults `4`. Isolation at depth
///   strictly less than this fires `Warn`.
/// - `seed`: i64 — defaults `0xC0FFEE`. Determinism is a feature.
pub struct IsolationForestLight {
    schema: RuleSchema,
}

impl IsolationForestLight {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("finance", "tx.isolation-forest-light", 1))
                .with_tags(finance_tags("isolation-forest-light")),
        }
    }
}

impl Default for IsolationForestLight {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for IsolationForestLight {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let value = match input.param("value").and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => return RuleOutput::Assertion(missing(&self.schema.id, "value")),
        };
        let population: Vec<f64> = input
            .param("population")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.as_f64())
            .collect();
        if population.len() < 4 {
            return RuleOutput::Assertion(
                Verdict::healthy(
                    self.schema.id.clone(),
                    Utc::now(),
                    "iforest-light skipped: population < 4 samples".to_owned(),
                )
                .with_coverage(degraded_coverage()),
            );
        }
        let max_depth = input
            .param("max_depth")
            .and_then(|v| v.as_i64())
            .unwrap_or(8)
            .clamp(1, 32) as u32;
        let threshold_depth = input
            .param("threshold_depth")
            .and_then(|v| v.as_i64())
            .unwrap_or(4)
            .clamp(1, 32) as u32;
        let seed = input
            .param("seed")
            .and_then(|v| v.as_i64())
            .unwrap_or(0x00C0_FFEE) as u64;

        // Single tree, deterministic. Walk: at each level pick a
        // pivot in [min, max] of the surviving partition using a
        // splitmix64 PRNG; recurse into whichever half `value`
        // falls in.
        let mut bucket: Vec<f64> = population.clone();
        let mut state = seed;
        let mut depth: u32 = 0;
        while depth < max_depth && bucket.len() > 1 {
            let (lo, hi) = bucket
                .iter()
                .copied()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
                    (a.min(x), b.max(x))
                });
            if !(hi > lo) {
                break;
            }
            state = splitmix64(state);
            let frac = ((state >> 11) as f64) / ((1u64 << 53) as f64);
            let pivot = lo + frac * (hi - lo);
            let go_left = value < pivot;
            bucket.retain(|x| (*x < pivot) == go_left);
            depth += 1;
        }
        let now = Utc::now();
        let v = if depth < threshold_depth {
            Verdict::warn(
                self.schema.id.clone(),
                now,
                format!("iforest-light: isolated at depth {depth} < threshold {threshold_depth}"),
            )
        } else {
            Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("iforest-light: isolated at depth {depth} >= threshold {threshold_depth}"),
            )
        };
        RuleOutput::Assertion(v)
    }
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

// --------------------------------------------------------------------
// finance.tx.duplicate@1
// --------------------------------------------------------------------

/// `finance.tx.duplicate@1` — flags duplicate-looking transactions.
///
/// Inputs (`params`):
/// - `transactions`: JSON array of
///   `{"account": str, "amount": number, "ts": i64-secs }`.
/// - `bucket_secs`: i64 — defaults `60`. Two transactions whose
///   `ts` differ by less than `bucket_secs` and whose
///   `(account, amount)` match are treated as duplicates.
///
/// Outcome: `Critical` if any duplicate group has size > 1,
/// `Healthy` otherwise. The verdict summary names the offending
/// account / amount.
pub struct DuplicateTx {
    schema: RuleSchema,
}

impl DuplicateTx {
    /// Construct the rule.
    pub fn new() -> Self {
        Self {
            schema: RuleSchema::assertion(RuleId::new("finance", "tx.duplicate", 1))
                .with_tags(finance_tags("duplicate-tx")),
        }
    }
}

impl Default for DuplicateTx {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Rule for DuplicateTx {
    fn schema(&self) -> &RuleSchema {
        &self.schema
    }

    async fn evaluate(&self, input: RuleInput) -> RuleOutput {
        let txs = input
            .param("transactions")
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default();
        if txs.is_empty() {
            return RuleOutput::Assertion(
                Verdict::healthy(
                    self.schema.id.clone(),
                    Utc::now(),
                    "duplicate-tx: empty window".to_owned(),
                )
                .with_coverage(degraded_coverage()),
            );
        }
        let bucket_secs = input
            .param("bucket_secs")
            .and_then(|v| v.as_i64())
            .unwrap_or(60)
            .max(1);
        // Group by (account, amount-cents, ts/bucket_secs).
        let mut groups: BTreeMap<(String, i64, i64), Vec<i64>> = BTreeMap::new();
        for t in &txs {
            let account = t
                .get("account")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_owned();
            let amount = t.get("amount").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
            let ts = t.get("ts").and_then(|v| v.as_i64()).unwrap_or(0);
            if amount.is_nan() || account.is_empty() {
                continue;
            }
            // Bucket the amount to cents so float jitter doesn't
            // mask a true dup; bucket the timestamp to
            // `bucket_secs`.
            let cents = (amount * 100.0).round() as i64;
            let bucket = ts.div_euclid(bucket_secs);
            groups.entry((account, cents, bucket)).or_default().push(ts);
        }
        let dup = groups
            .iter()
            .find(|(_, ts_list)| ts_list.len() > 1)
            .map(|(k, v)| (k.clone(), v.len()));
        let now = Utc::now();
        let v = match dup {
            Some(((account, cents, _bucket), count)) => Verdict::critical(
                self.schema.id.clone(),
                now,
                format!(
                    "duplicate-tx: account={account} amount={:.2} appeared {count}× within {bucket_secs}s",
                    cents as f64 / 100.0
                ),
            )
            .with_coverage(coverage_with_quality(
                QualityFlagId::new("finance.quality", "duplicate-timestamp", 1),
                QualityFlagSeverity::Warn,
            )),
            None => Verdict::healthy(
                self.schema.id.clone(),
                now,
                format!("duplicate-tx: {} txs, no dups within {bucket_secs}s", txs.len()),
            ),
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

fn degraded_coverage() -> Coverage {
    let mut c = Coverage::full_point();
    c.quality_flags.push(QualityFlag::new(
        QualityFlagId::new("starter.quality", "gap", 1),
        QualityFlagSeverity::Info,
    ));
    c
}

fn coverage_with_quality(id: QualityFlagId, sev: QualityFlagSeverity) -> Coverage {
    let mut c = Coverage::full_point();
    c.quality_flags.push(QualityFlag::new(id, sev));
    c
}

/// Boxed `Rule` impls for the host's `RuleRegistry::register()` loop.
pub fn rules() -> Vec<Arc<dyn Rule>> {
    vec![
        Arc::new(ZScore::new()),
        Arc::new(IsolationForestLight::new()),
        Arc::new(DuplicateTx::new()),
    ]
}

/// `finance.quality.*` flag descriptors. Each entry is
/// `(QualityFlagId, description, remediation)`.
pub fn quality_flags() -> Vec<(QualityFlagId, &'static str, &'static str)> {
    vec![
        (
            QualityFlagId::new("finance.quality", "duplicate-timestamp", 1),
            "two transactions for the same account / amount landed within the dedupe bucket — one is likely a replay",
            "investigate the gateway / webhook source; widen `bucket_secs` only if the source is provably idempotent",
        ),
        (
            QualityFlagId::new("finance.quality", "fx-rate-stale", 1),
            "FX rate older than the configured freshness window — amount conversions may be off",
            "refresh the FX feed; chain a `derive` rule that normalises currency before the assertion",
        ),
    ]
}

/// Convenience: a `QualityFlag` value for the duplicate-timestamp flag.
pub fn duplicate_timestamp_flag() -> QualityFlag {
    QualityFlag::new(
        QualityFlagId::new("finance.quality", "duplicate-timestamp", 1),
        QualityFlagSeverity::Warn,
    )
}
