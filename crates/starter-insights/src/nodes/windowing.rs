//! `starter.flow.window.tumble` and `starter.flow.window.slide` —
//! windowing node bodies (Insights SCOPE Phase 2).
//!
//! Both nodes consume a flat sample stream and emit one or more
//! [`Dataset`] payloads aligned to `[start, end)` half-open windows.
//! Time zone is mandatory (R-ins-6) — windows align to the
//! configured IANA `tz`, not to UTC, so "midnight London"
//! windowing is DST-correct.
//!
//! Input slots (common):
//! - `samples` ([`SlotValue::Json`], required) — a JSON array of
//!   `{ "ts": <RFC3339 / unix-ms>, "value": <number>, ... }`
//!   objects. Samples without a parseable `ts` are dropped and
//!   counted against `samples_expected` only.
//! - `tz` ([`SlotValue::String`], optional) — IANA tz; defaults to
//!   `"UTC"`.
//! - `size_secs` ([`SlotValue::Int`], required) — tumble/slide
//!   window size in seconds (e.g. 3600 for a 1h window).
//! - `step_secs` ([`SlotValue::Int`], `window.slide` only) — slide
//!   step in seconds. Must be `< size_secs`; equal means tumbling.
//! - `expected_per_window` ([`SlotValue::Int`], optional) — used by
//!   the per-window `raw.samples_expected` field. Defaults to the
//!   number of samples observed in the window (full coverage).
//!
//! Output slot:
//! - `windows` ([`SlotValue::Json`]) — a JSON array of serialised
//!   [`Dataset`]s, one per emitted window, oldest first.

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};
use starter_spi::insights::{
    Coverage, Dataset, DatasetSchema, RawCoverage, TimeZoneId, VecDatasetRows, Window,
};
use std::sync::Arc;

/// Reverse-DNS kind id for the tumbling window.
pub const TUMBLE_KIND_ID: &str = "starter.flow.window.tumble";

/// Reverse-DNS kind id for the sliding window.
pub const SLIDE_KIND_ID: &str = "starter.flow.window.slide";

/// Input slot carrying the flat sample array.
pub const SAMPLES_SLOT: &str = "samples";

/// Input slot — IANA timezone (e.g. `"Europe/London"`).
pub const TZ_SLOT: &str = "tz";

/// Input slot — window size in seconds.
pub const SIZE_SECS_SLOT: &str = "size_secs";

/// Input slot — slide step in seconds.
pub const STEP_SECS_SLOT: &str = "step_secs";

/// Input slot — declared expected sample count per window. Drives
/// `RawCoverage::samples_expected` (R-ins-6); defaults to the number
/// of samples observed in the window.
pub const EXPECTED_PER_WINDOW_SLOT: &str = "expected_per_window";

/// Output slot carrying a JSON array of [`Dataset`] payloads.
pub const WINDOWS_SLOT: &str = "windows";

/// Tumbling-window node body.
pub struct WindowTumbleNode {
    kind: KindId,
}

impl WindowTumbleNode {
    /// Construct a new `window.tumble` body.
    pub fn new() -> Self {
        Self {
            kind: KindId::new(TUMBLE_KIND_ID).expect("kind id is valid"),
        }
    }
}

impl Default for WindowTumbleNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeBehavior for WindowTumbleNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let cfg = match parse_cfg(&input, /* slide */ false) {
            Ok(c) => c,
            Err(msg) => return Err(NodeError::InvalidInput(msg)),
        };
        Ok(emit(&cfg))
    }
}

/// Sliding-window node body.
pub struct WindowSlideNode {
    kind: KindId,
}

impl WindowSlideNode {
    /// Construct a new `window.slide` body.
    pub fn new() -> Self {
        Self {
            kind: KindId::new(SLIDE_KIND_ID).expect("kind id is valid"),
        }
    }
}

impl Default for WindowSlideNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NodeBehavior for WindowSlideNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        let cfg = match parse_cfg(&input, /* slide */ true) {
            Ok(c) => c,
            Err(msg) => return Err(NodeError::InvalidInput(msg)),
        };
        Ok(emit(&cfg))
    }
}

struct Cfg {
    samples: Vec<(DateTime<Utc>, serde_json::Value)>,
    tz: Tz,
    tz_id: TimeZoneId,
    size_secs: i64,
    step_secs: i64,
    expected_per_window: Option<u64>,
}

fn parse_cfg(input: &SlotMap, slide: bool) -> Result<Cfg, String> {
    let arr = match input.get(SAMPLES_SLOT) {
        Some(SlotValue::Json(serde_json::Value::Array(a))) => a.clone(),
        _ => return Err(format!("missing `{SAMPLES_SLOT}` (JSON array)")),
    };
    let size_secs = match input.get(SIZE_SECS_SLOT) {
        Some(SlotValue::Int(n)) if *n > 0 => *n,
        _ => return Err(format!("missing positive `{SIZE_SECS_SLOT}`")),
    };
    let step_secs = if slide {
        match input.get(STEP_SECS_SLOT) {
            Some(SlotValue::Int(n)) if *n > 0 && *n <= size_secs => *n,
            _ => {
                return Err(format!(
                    "window.slide: missing `{STEP_SECS_SLOT}` in (0, size_secs]"
                ))
            }
        }
    } else {
        size_secs
    };
    let tz_id = match input.get(TZ_SLOT) {
        Some(SlotValue::String(s)) => TimeZoneId::new(s.clone()),
        _ => TimeZoneId::utc(),
    };
    let tz: Tz = tz_id
        .as_str()
        .parse()
        .map_err(|_| format!("unknown IANA tz `{}`", tz_id.as_str()))?;
    let expected_per_window = match input.get(EXPECTED_PER_WINDOW_SLOT) {
        Some(SlotValue::Int(n)) if *n > 0 => Some(*n as u64),
        _ => None,
    };

    let mut samples: Vec<(DateTime<Utc>, serde_json::Value)> = arr
        .into_iter()
        .filter_map(|s| {
            let ts = extract_ts(&s)?;
            Some((ts, s))
        })
        .collect();
    samples.sort_by_key(|(t, _)| *t);

    Ok(Cfg {
        samples,
        tz,
        tz_id,
        size_secs,
        step_secs,
        expected_per_window,
    })
}

fn extract_ts(v: &serde_json::Value) -> Option<DateTime<Utc>> {
    let ts = v.get("ts")?;
    if let Some(s) = ts.as_str() {
        return DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc));
    }
    if let Some(ms) = ts.as_i64() {
        return Some(Utc.timestamp_millis_opt(ms).single()?);
    }
    None
}

fn emit(cfg: &Cfg) -> SlotMap {
    let mut out_windows: Vec<serde_json::Value> = Vec::new();
    if cfg.samples.is_empty() {
        let mut m = SlotMap::new();
        m.insert(
            WINDOWS_SLOT.to_owned(),
            SlotValue::Json(serde_json::Value::Array(out_windows)),
        );
        return m;
    }
    let first = cfg.samples.first().unwrap().0;
    let last = cfg.samples.last().unwrap().0;
    // Anchor window starts on midnight of the configured tz, so DST
    // transitions land on a boundary (R-ins-6).
    let anchor = anchor_for(&cfg.tz, first);
    let mut start = anchor;
    while start + chrono::Duration::seconds(cfg.size_secs) <= first {
        start += chrono::Duration::seconds(cfg.step_secs);
    }

    while start <= last {
        let end = start + chrono::Duration::seconds(cfg.size_secs);
        let rows: Vec<serde_json::Value> = cfg
            .samples
            .iter()
            .filter(|(t, _)| *t >= start && *t < end)
            .map(|(_, v)| v.clone())
            .collect();
        let samples_present = rows.len() as u64;
        let samples_expected = cfg.expected_per_window.unwrap_or(samples_present.max(1));
        let confidence = if samples_expected == 0 {
            1.0
        } else {
            (samples_present as f32 / samples_expected as f32).clamp(0.0, 1.0)
        };
        let raw = RawCoverage::new(samples_expected, samples_present, confidence);
        let coverage = Coverage::from_raw(raw);
        let dataset = Dataset::from_parts(
            DatasetSchema::new(["ts", "value"]),
            Arc::new(VecDatasetRows::new(rows)),
            coverage,
            cfg.tz_id.clone(),
            Some(Window::new(start, end)),
        );
        out_windows.push(serde_dataset(&dataset));
        start += chrono::Duration::seconds(cfg.step_secs);
    }

    let mut m = SlotMap::new();
    m.insert(
        WINDOWS_SLOT.to_owned(),
        SlotValue::Json(serde_json::Value::Array(out_windows)),
    );
    m
}

fn anchor_for(tz: &Tz, first: DateTime<Utc>) -> DateTime<Utc> {
    // Midnight of the same date as `first` in the configured zone,
    // converted back to UTC. This makes daily / hourly windows align
    // to local midnight regardless of the UTC clock.
    let local = first.with_timezone(tz);
    let local_midnight = tz
        .with_ymd_and_hms(local.year(), local.month(), local.day(), 0, 0, 0)
        .single()
        .unwrap_or(local);
    local_midnight.with_timezone(&Utc)
}

use chrono::Datelike;

fn serde_dataset(d: &Dataset) -> serde_json::Value {
    // Datasets are not serde::Serialize on the SPI; serialise a
    // bounded JSON projection that downstream nodes (rule.rust /
    // verdict.join) can consume.
    serde_json::json!({
        "schema": { "columns": d.schema.columns },
        "rows":   d.rows.snapshot(),
        "coverage": {
            "raw": {
                "samples_expected": d.coverage.raw.samples_expected,
                "samples_present":  d.coverage.raw.samples_present,
                "confidence":       d.coverage.raw.confidence,
            },
            "effective": {
                "confidence":    d.coverage.effective.confidence,
                "penalty_chain": d.coverage.effective.penalty_chain
                    .iter().map(|(id, p)| serde_json::json!([id.to_string(), p])).collect::<Vec<_>>(),
            },
            "quality_flags": d.coverage.quality_flags.iter().map(|f| {
                serde_json::json!({"id": f.id.to_string(), "severity": format!("{:?}", f.severity)})
            }).collect::<Vec<_>>(),
        },
        "tz": d.tz.as_str(),
        "window": d.window.as_ref().map(|w| serde_json::json!({
            "start": w.start.to_rfc3339(),
            "end":   w.end.to_rfc3339(),
        })),
    })
}
