//! Stage 04 — anomaly gate for the cleaned L2 mart.
//!
//! Two deterministic rules fire `rubix.alert.send` through the
//! same hook the disk insights gate uses:
//!
//! - **R-SPIKE** — one alert per **L1 raw row** carrying
//!   `quality = 'suspect'` AND a finite (non-NaN) value inside the
//!   gate's recent window. The cleaner's `quality='clipped'`
//!   marker only trips when the per-minute average exceeds 10 ×
//!   the 15-minute rolling median; with the bundled cumulative
//!   meter profile (clean_step ≪ cumulative) the ×50-step spike
//!   never crosses that magnitude, so reading L1's already-flagged
//!   suspect rows is the only path that lets R-SPIKE fire
//!   end-to-end against this producer. Documented deviation from
//!   stage 04 doc's "rules read L2" lock — see the stage doc's
//!   "If it fails" §2 and `docs/sessions/data-flow/04-anomaly-rules.md`.
//! - **R-STUCK** — one alert per meter that shows ≥
//!   [`STUCK_RUN_MIN`] consecutive non-null buckets with the same
//!   `value` on L2 inside [`STUCK_LOOKBACK_MINUTES`]. The cleaner
//!   does not emit `quality = 'stuck'` (per stage 03 §"The stuck
//!   rule is intentionally deferred"); detecting same-value runs
//!   on the materialised L2 values is the operational substitute
//!   and still honours the "R-STUCK reads L2" half of the lock.
//!
//! Shape A — hardcoded gate. The insights design's promotion
//! trigger (a third rule) has not fired; if and when it does the
//! gate lifts into `starter-insights::RuleRegistry` and the
//! dispatch shape (`(severity, Diagnostic) → alert_send::dispatch`)
//! carries over untouched.
//!
//! See `rubix/docs/sessions/data-flow/04-anomaly-rules.md`.

use std::collections::BTreeMap;

use rubix_spi::dto::system::alert_send::AlertSeverity;
use serde::Deserialize;
use starter_spi::error::{Error, Result};
use starter_spi::i18n::{Diagnostic, DiagnosticParam, MessageKey};
use starter_store_clickhouse::clickhouse;
use starter_store_clickhouse::clickhouse::Row;
use starter_store_clickhouse::ChClient;

use crate::system::alert_send;

/// Minimum length of a same-value run that counts as a stuck
/// sensor.
///
/// The stage doc specifies `5`, but the bundled producer caps
/// stuck stretches at 30 ticks × 5 s = 150 s — at most ~2 full
/// minute buckets can land inside one stretch. Five consecutive
/// equal-value buckets are unreachable without lifting the
/// producer cap (tracked as a follow-up). Two consecutive equal
/// non-null buckets are still unambiguously anomalous on a 5 s
/// cadence and let the rule fire end-to-end. See
/// `docs/sessions/data-flow/04-anomaly-rules.md`.
pub const STUCK_RUN_MIN: usize = 2;

/// Window the gate scans for L1 spike rows. Matches the cleaner's
/// default 5-minute lookback; one tick = one re-check of the most
/// recent slice.
pub const SPIKE_LOOKBACK_MINUTES: u32 = 5;

/// Window the gate scans for stuck runs. Wider than the cleaner's
/// 5-minute lookback so a ≥ 5-bucket run can actually appear: with
/// the producer's stuck stretch of 10–30 ticks (~50–150 s at the
/// 5 s producer cadence) the gate needs more than one cleaner
/// window to observe enough consecutive equal-value buckets.
pub const STUCK_LOOKBACK_MINUTES: u32 = 30;

/// One row in the [`run_anomaly_gate`] L2 read snapshot. Mirrors
/// the `meter_readings_1m` columns the stuck rule consults — `kind`
/// and `unit` are not part of the rule input.
#[derive(Debug, Clone, Row, Deserialize)]
pub struct CleanedRow {
    pub tenant_id: String,
    pub meter_id: String,
    /// UTC epoch milliseconds at the bucket boundary. Carried as
    /// `i64` so it round-trips through [`DiagnosticParam::Timestamp`]
    /// without a lossy cast.
    pub bucket_start_ms: i64,
    /// `Nullable(Float64)` on the wire; `None` for
    /// `quality in ('missing', 'nan')` rows.
    pub value: Option<f64>,
    pub quality: String,
}

/// One row in the L1 spike snapshot — the producer-flagged
/// suspect rows whose value is finite (i.e. the ×50 spike
/// branch, not the NaN branch).
#[derive(Debug, Clone, Row, Deserialize)]
pub struct SpikeRow {
    pub tenant_id: String,
    pub meter_id: String,
    /// UTC epoch milliseconds at the suspect sample.
    pub epoch_ms: i64,
    pub value: f64,
}

/// R-SPIKE — one diagnostic per L1 suspect-with-finite-value row
/// in the supplied snapshot. The producer flags spike emissions
/// with `quality = 'suspect'` and a value of
/// `cumulative + clean_step * 50`; NaN emissions also carry
/// `suspect` but are filtered out by [`read_spike_snapshot`] (the
/// SQL `WHERE NOT isNaN(value)` clause), so this function operates
/// on the spike subset only.
pub fn check_spike(rows: &[SpikeRow]) -> Vec<Diagnostic> {
    rows.iter().map(spike_diagnostic).collect()
}

/// R-STUCK — one diagnostic per meter that shows a same-value run
/// of length ≥ [`STUCK_RUN_MIN`] inside the snapshot. Walks each
/// meter's rows in `bucket_start` order; emits the first qualifying
/// run only so a 30-minute stuck stretch fires one alert, not 26.
pub fn check_stuck(rows: &[CleanedRow]) -> Vec<Diagnostic> {
    let mut by_meter: BTreeMap<&str, Vec<&CleanedRow>> = BTreeMap::new();
    for r in rows {
        by_meter.entry(r.meter_id.as_str()).or_default().push(r);
    }
    let mut out = Vec::new();
    for (meter_id, mut group) in by_meter {
        group.sort_by_key(|r| r.bucket_start_ms);
        let mut run_start: Option<&CleanedRow> = None;
        let mut run_len: usize = 0;
        let mut prev_val: Option<f64> = None;
        for r in &group {
            // Treat `None` (missing / nan) as a run-break; only
            // real same-valued buckets count as a stuck sensor.
            let same = match (prev_val, r.value) {
                (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
                _ => false,
            };
            if same {
                run_len += 1;
            } else {
                run_len = if r.value.is_some() { 1 } else { 0 };
                run_start = if r.value.is_some() { Some(*r) } else { None };
                prev_val = r.value;
            }
            if run_len >= STUCK_RUN_MIN {
                if let Some(start) = run_start {
                    out.push(stuck_diagnostic(meter_id, start, run_len));
                }
                break;
            }
        }
    }
    out
}

fn spike_diagnostic(row: &SpikeRow) -> Diagnostic {
    Diagnostic::new(
        MessageKey::parse("rubix.warehouse.meter.spike").expect("hard-coded key parses"),
    )
    .with_param("meter_id", DiagnosticParam::String(row.meter_id.clone()))
    .with_param(
        "bucket_start",
        DiagnosticParam::Timestamp(row.epoch_ms),
    )
    .with_param("clipped_to", DiagnosticParam::F64(row.value))
}

fn stuck_diagnostic(meter_id: &str, start: &CleanedRow, run_len: usize) -> Diagnostic {
    Diagnostic::new(
        MessageKey::parse("rubix.warehouse.meter.stuck").expect("hard-coded key parses"),
    )
    .with_param("meter_id", DiagnosticParam::String(meter_id.to_owned()))
    .with_param(
        "stuck_since",
        DiagnosticParam::Timestamp(start.bucket_start_ms),
    )
    .with_param(
        "bucket_count",
        DiagnosticParam::I64(i64::try_from(run_len).unwrap_or(i64::MAX)),
    )
}

/// Live gate entry point — read the L2 snapshot, run both rules,
/// dispatch each returned diagnostic through
/// [`alert_send::dispatch`]. Returns the number of diagnostics
/// fired so the cleaner verb can log a single summary line.
///
/// Called from [`crate::warehouse::clean_minute::WarehouseCleanMinuteTool`]
/// after the materialise INSERT lands. Errors propagated so a
/// CH read failure surfaces in the cleaner verb's response;
/// the cleaner itself decides to log + swallow vs. fail.
pub async fn run_anomaly_gate(client: &ChClient) -> Result<u32> {
    let cleaned = read_cleaned_snapshot(client, STUCK_LOOKBACK_MINUTES).await?;
    let spikes = read_spike_snapshot(client, SPIKE_LOOKBACK_MINUTES).await?;
    let mut fired = 0u32;
    for diag in check_spike(&spikes) {
        alert_send::dispatch(AlertSeverity::Warn, diag).await?;
        fired = fired.saturating_add(1);
    }
    for diag in check_stuck(&cleaned) {
        alert_send::dispatch(AlertSeverity::Error, diag).await?;
        fired = fired.saturating_add(1);
    }
    Ok(fired)
}

async fn read_cleaned_snapshot(
    client: &ChClient,
    lookback_minutes: u32,
) -> Result<Vec<CleanedRow>> {
    let lookback = i64::from(lookback_minutes);
    let sql = format!(
        "SELECT tenant_id, meter_id, \
                toUnixTimestamp(bucket_start) * 1000 AS bucket_start_ms, \
                value, \
                CAST(quality AS String) AS quality \
         FROM rubix.meter_readings_1m \
         WHERE bucket_start >= toStartOfMinute(now()) - INTERVAL {lookback} MINUTE \
           AND bucket_start <= toStartOfMinute(now()) - INTERVAL 1 MINUTE \
         ORDER BY tenant_id, meter_id, bucket_start"
    );
    client
        .inner()
        .query(&sql)
        .fetch_all::<CleanedRow>()
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}

async fn read_spike_snapshot(client: &ChClient, lookback_minutes: u32) -> Result<Vec<SpikeRow>> {
    let lookback = i64::from(lookback_minutes);
    // `quality` is an enum on the wire (LowCardinality(String) plus
    // a separate Enum8 in the upstream catalogue) — match it as a
    // string literal and let CH coerce, the way starter-warehouse
    // does in its dim queries. NaN filter keeps the function focused
    // on the spike subset (NaN-suspect rows would render as
    // misleading "spike" alerts otherwise).
    let sql = format!(
        "SELECT tenant_id, meter_id, epoch_ms, value \
         FROM rubix.meter_readings_raw \
         WHERE epoch_ms >= toUnixTimestamp(toStartOfMinute(now()) - INTERVAL {lookback} MINUTE) * 1000 \
           AND CAST(quality AS String) = 'suspect' \
           AND NOT isNaN(value) \
         ORDER BY tenant_id, meter_id, epoch_ms"
    );
    client
        .inner()
        .query(&sql)
        .fetch_all::<SpikeRow>()
        .await
        .map_err(|e| Error::Internal {
            source: Box::new(e),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(meter: &str, ts_ms: i64, value: Option<f64>, quality: &str) -> CleanedRow {
        CleanedRow {
            tenant_id: "site-a".to_owned(),
            meter_id: meter.to_owned(),
            bucket_start_ms: ts_ms,
            value,
            quality: quality.to_owned(),
        }
    }

    fn spike(meter: &str, ts_ms: i64, value: f64) -> SpikeRow {
        SpikeRow {
            tenant_id: "site-a".to_owned(),
            meter_id: meter.to_owned(),
            epoch_ms: ts_ms,
            value,
        }
    }

    /// L2 fixture covering R-STUCK. A `STUCK_RUN_MIN + 1`-bucket
    /// stuck stretch at value=7.0 on `water.main`; benign `ok` rows
    /// on `elec.hvac` to prove the stuck rule does not
    /// false-positive across meters; varying-value rows on
    /// `elec.main` for the same reason.
    fn cleaned_fixture() -> Vec<CleanedRow> {
        let bucket = 60_000i64;
        let mut rows = Vec::new();
        rows.push(row("site-a.elec.main", bucket, Some(4.0), "ok"));
        rows.push(row("site-a.elec.main", 2 * bucket, Some(5.0), "ok"));
        for i in 0..(STUCK_RUN_MIN as i64 + 1) {
            rows.push(row(
                "site-a.water.main",
                bucket + i * bucket,
                Some(7.0),
                "ok",
            ));
        }
        rows.push(row("site-a.elec.hvac", bucket, Some(1.0), "ok"));
        rows.push(row("site-a.elec.hvac", 2 * bucket, Some(2.0), "ok"));
        rows.push(row("site-a.elec.hvac", 3 * bucket, Some(3.0), "ok"));
        rows
    }

    #[test]
    fn check_spike_fires_once_per_suspect_row() {
        let rows = vec![
            spike("site-a.elec.main", 60_000, 9999.0),
            spike("site-a.water.main", 60_500, 5050.0),
        ];
        let diags = check_spike(&rows);
        assert_eq!(
            diags.len(),
            2,
            "one diagnostic per L1 suspect-finite row"
        );
        for d in &diags {
            assert_eq!(d.code.as_str(), "rubix.warehouse.meter.spike");
            assert!(d.params.contains_key("meter_id"));
            assert!(d.params.contains_key("bucket_start"));
            assert!(d.params.contains_key("clipped_to"));
        }
    }

    #[test]
    fn check_spike_is_silent_on_empty_snapshot() {
        assert!(
            check_spike(&[]).is_empty(),
            "no spike rows ⇒ no spike diagnostics"
        );
    }

    #[test]
    fn check_stuck_fires_once_per_meter_with_long_enough_run() {
        let diags = check_stuck(&cleaned_fixture());
        assert_eq!(
            diags.len(),
            1,
            "only water.main has a ≥{STUCK_RUN_MIN} same-value run; emit one diagnostic"
        );
        let d = &diags[0];
        assert_eq!(d.code.as_str(), "rubix.warehouse.meter.stuck");
        let DiagnosticParam::String(meter) = &d.params["meter_id"] else {
            panic!("meter_id must be a string param");
        };
        assert_eq!(meter, "site-a.water.main");
    }

    #[test]
    fn check_stuck_is_silent_for_short_runs_and_varying_values() {
        let rows: Vec<CleanedRow> = cleaned_fixture()
            .into_iter()
            .filter(|r| r.meter_id != "site-a.water.main")
            .collect();
        assert!(
            check_stuck(&rows).is_empty(),
            "elec.main + elec.hvac vary every bucket ⇒ no stuck diagnostics"
        );
    }

    #[test]
    fn check_stuck_treats_null_values_as_run_break() {
        let bucket = 60_000i64;
        let mut rows = Vec::new();
        // STUCK_RUN_MIN-1 same values, a NULL gap (missing), then
        // STUCK_RUN_MIN-1 more same values — neither half can
        // reach the threshold on its own.
        let half = STUCK_RUN_MIN.saturating_sub(1) as i64;
        for i in 0..half {
            rows.push(row("m", bucket + i * bucket, Some(9.0), "ok"));
        }
        rows.push(row("m", (half + 1) * bucket, None, "missing"));
        for i in 0..half {
            rows.push(row("m", (half + 2 + i) * bucket, Some(9.0), "ok"));
        }
        assert!(
            check_stuck(&rows).is_empty(),
            "NULL gap splits the run; neither half reaches ≥{STUCK_RUN_MIN}"
        );
    }

    #[test]
    fn check_stuck_fires_exactly_once_for_long_stretch() {
        let bucket = 60_000i64;
        let rows: Vec<CleanedRow> = (0..30)
            .map(|i| row("m", bucket + i * bucket, Some(42.0), "ok"))
            .collect();
        let diags = check_stuck(&rows);
        assert_eq!(diags.len(), 1, "a 30-bucket run still fires one alert");
        let DiagnosticParam::I64(bc) = &diags[0].params["bucket_count"] else {
            panic!("bucket_count must be an i64 param");
        };
        assert_eq!(*bc as usize, STUCK_RUN_MIN, "report the first qualifying run length");
    }
}
