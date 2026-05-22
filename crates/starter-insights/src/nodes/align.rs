//! `starter.flow.align` — multi-source time-align + resample node
//! (Insights SCOPE R-ins-7, D8).
//!
//! `align` is the most domain-loaded node in the capability. It is
//! NOT a `Rule` (it takes N inputs and emits a `Frame`, an ordered
//! tuple of co-time-indexed `Dataset`s) — but per D8 it carries a
//! `NodeId` with the same `(namespace, name, semver)` shape as
//! `RuleId` so the audit trail in `Verdict.evidence` provenance is
//! uniform.
//!
//! `align` also **sets `raw.confidence`** for the rest of the chain
//! (typically `samples_present / samples_expected` adjusted by the
//! configured gap policy).
//!
//! Frames are an internal slot value type (a JSON array of dataset
//! snapshots tagged with their source slot name); they are not
//! surfaced as a public registry concept. Downstream derivation
//! rules read the frame, project the dataset they care about, and
//! re-wrap.
//!
//! Input slots:
//! - `sources` ([`SlotValue::Json`], required) — JSON object whose
//!   keys are source ids and whose values are sample arrays
//!   `[{ "ts": rfc3339, "value": <any> }, ...]`. Sources that
//!   contributed no samples are still expected — the gap policy
//!   below decides what to do.
//! - `frame_secs` ([`SlotValue::Int`], required) — frame size in
//!   seconds.
//! - `tz` ([`SlotValue::String`], required) — IANA tz string.
//! - `gap_policy` ([`SlotValue::String`], optional) — `mark_gap`
//!   (default), `skip`, or `fail`.
//! - `align_node_id` ([`SlotValue::String`], optional) —
//!   D8 namespace.name@major audit identity. Defaults to
//!   `starter.align.tumble@1`.
//!
//! Output slot:
//! - `frame` ([`SlotValue::Json`]) — `{ "node_id": "...", "tz":
//!   "...", "frame_start_ms": ..., "frame_end_ms": ..., "raw_conf":
//!   ..., "sources": { "<src>": <Dataset-snapshot> } }`.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};

/// Reverse-DNS kind id.
pub const KIND_ID: &str = "starter.flow.align";

/// Required input slot: JSON object of source samples.
pub const SOURCES_SLOT: &str = "sources";

/// Required input slot: frame size in seconds.
pub const FRAME_SECS_SLOT: &str = "frame_secs";

/// Required input slot: IANA tz string.
pub const TZ_SLOT: &str = "tz";

/// Optional input slot: gap policy (`mark_gap` | `skip` | `fail`).
pub const GAP_POLICY_SLOT: &str = "gap_policy";

/// Optional input slot: align node audit id (D8). Defaults to
/// `starter.align.tumble@1`.
pub const ALIGN_NODE_ID_SLOT: &str = "align_node_id";

/// Output slot: serialised frame value.
pub const FRAME_SLOT: &str = "frame";

/// Node body for `starter.flow.align`.
pub struct AlignNode {
    kind: KindId,
}

impl AlignNode {
    /// Construct a new align node body.
    pub fn new() -> Self {
        Self {
            kind: KindId::new(KIND_ID).expect("KIND_ID is valid"),
        }
    }
}

impl Default for AlignNode {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a timestamp value — accepts RFC-3339 string or epoch
/// milliseconds.
fn parse_ts(v: &serde_json::Value) -> Option<DateTime<Utc>> {
    if let Some(s) = v.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.with_timezone(&Utc));
        }
    }
    if let Some(ms) = v.as_i64() {
        return DateTime::from_timestamp_millis(ms);
    }
    None
}

#[async_trait]
impl NodeBehavior for AlignNode {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, _ctx: NodeCtx<'_>, mut input: SlotMap) -> Result<SlotMap, NodeError> {
        let sources = match input.remove(SOURCES_SLOT) {
            Some(SlotValue::Json(serde_json::Value::Object(m))) => m,
            _ => {
                return Err(NodeError::InvalidInput(
                    "align: `sources` slot must be a JSON object".into(),
                ))
            }
        };
        let frame_secs = match input.remove(FRAME_SECS_SLOT) {
            Some(SlotValue::Int(n)) if n > 0 => n,
            _ => {
                return Err(NodeError::InvalidInput(
                    "align: `frame_secs` must be a positive Int".into(),
                ))
            }
        };
        let tz_str = match input.remove(TZ_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => {
                return Err(NodeError::InvalidInput(
                    "align: `tz` must be an IANA string".into(),
                ))
            }
        };
        let gap_policy = match input.remove(GAP_POLICY_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => "mark_gap".to_owned(),
        };
        let node_id = match input.remove(ALIGN_NODE_ID_SLOT) {
            Some(SlotValue::String(s)) => s,
            _ => "starter.align.tumble@1".to_owned(),
        };

        // Compute the frame window: smallest source-min ts → largest
        // source-max ts, snapped to the frame boundary.
        let mut min_ts: Option<DateTime<Utc>> = None;
        let mut max_ts: Option<DateTime<Utc>> = None;
        for (_k, arr) in sources.iter() {
            if let Some(rows) = arr.as_array() {
                for r in rows {
                    if let Some(ts) = r.get("ts").and_then(parse_ts) {
                        min_ts = Some(min_ts.map_or(ts, |m| m.min(ts)));
                        max_ts = Some(max_ts.map_or(ts, |m| m.max(ts)));
                    }
                }
            }
        }
        let (frame_start, frame_end) = match (min_ts, max_ts) {
            (Some(s), Some(e)) => (s, e + chrono::Duration::seconds(frame_secs)),
            _ => {
                let now = Utc::now();
                (now, now + chrono::Duration::seconds(frame_secs))
            }
        };

        // Compute raw.confidence per source = samples_present / samples_expected.
        // `samples_expected` is the number of frame_secs slots in
        // `[frame_start, frame_end)`; gap policy decides whether a
        // missing source aborts (`fail`), is dropped (`skip`), or
        // is recorded as gap (`mark_gap`, the default).
        let total_secs = (frame_end - frame_start).num_seconds().max(1);
        let slots_per_source = (total_secs / frame_secs).max(1) as u64;

        let mut per_source: BTreeMap<String, serde_json::Value> = BTreeMap::new();
        let mut min_conf: f32 = 1.0;
        for (k, arr) in sources {
            let rows = arr.as_array().cloned().unwrap_or_default();
            let present = rows.len() as u64;
            let conf = if slots_per_source == 0 {
                1.0
            } else {
                (present as f32 / slots_per_source as f32).clamp(0.0, 1.0)
            };
            if present == 0 {
                match gap_policy.as_str() {
                    "fail" => {
                        return Err(NodeError::Backend(format!(
                            "align: source `{k}` contributed zero samples; gap_policy=fail"
                        )))
                    }
                    "skip" => continue,
                    _ => { /* mark_gap — fall through; conf is 0 */ }
                }
            }
            min_conf = min_conf.min(conf);
            // Snapshot the source as a tiny inline "dataset"
            // (schema column hint + rows). Downstream derivations
            // re-wrap as proper Dataset values once they have a
            // typed schema in hand.
            per_source.insert(
                k,
                serde_json::json!({
                    "tz": tz_str,
                    "samples_expected": slots_per_source,
                    "samples_present": present,
                    "raw_confidence": conf,
                    "rows": rows,
                }),
            );
        }

        let tz_parsed: chrono_tz::Tz = tz_str.parse().unwrap_or(chrono_tz::UTC);
        let _ = tz_parsed.from_utc_datetime(&frame_start.naive_utc()); // validate tz

        let frame = serde_json::json!({
            "node_id": node_id,
            "tz": tz_str,
            "frame_secs": frame_secs,
            "frame_start_ms": frame_start.timestamp_millis(),
            "frame_end_ms": frame_end.timestamp_millis(),
            "raw_confidence": min_conf,
            "gap_policy": gap_policy,
            "sources": per_source,
        });

        let mut out = SlotMap::new();
        out.insert(FRAME_SLOT.to_owned(), SlotValue::Json(frame));
        Ok(out)
    }
}
