//! `rubix.dataflow.synth.emit` — request/response DTOs and tool descriptor.
//!
//! Synthesises 0..N wire-shape meter readings for one tick. The mess
//! shapes (gap / spike / stuck / jitter / NaN) are knob-controlled and
//! seedable so the tool is unit-testable end-to-end without a flow
//! engine or a warehouse. See
//! `rubix/docs/sessions/data-flow/01-producer.md` for the framework
//! split (synthesis is a tool, delivery is a flow).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// `starter-authz` permission string. The dispatch wrapper passes
/// this to `Authz::check` before invoking. Synth is a dev/test tool
/// so it sits under the same `system.read` grant the other
/// observability verbs use.
pub const REQUIRED_PERMISSION: &str = "system.read";

/// Default mess knobs. Mirrored in the docs and used when a knob is
/// omitted from the request *and* its env-var fallback is unset.
pub const DEFAULT_SEED: u64 = 42;
pub const DEFAULT_GAP_PROB: f64 = 0.02;
pub const DEFAULT_SPIKE_PROB: f64 = 0.005;
pub const DEFAULT_STUCK_PROB: f64 = 0.001;
pub const DEFAULT_JITTER_MS: i64 = 20_000;
pub const DEFAULT_NAN_PROB: f64 = 0.0005;

/// Cumulative meter value range at process start. Both meters tick
/// upward from a value in this band so dashboards see plausible
/// numbers from the first read.
pub const ELEC_START_KWH: f64 = 10_000.0;
pub const WATER_START_L: f64 = 50_000.0;

/// Wire row emitted to downstream stages. Locked across stages
/// 01 → 05 (see `01-producer.md` "Wire shape").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct MeterReading {
    pub tenant_id: String,
    pub meter_id: String,
    pub kind: MeterKind,
    pub unit: MeterUnit,
    pub epoch_ms: i64,
    pub value: f64,
    pub quality: ReadingQuality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MeterKind {
    Electricity,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum MeterUnit {
    #[serde(rename = "kWh")]
    KWh,
    #[serde(rename = "L")]
    L,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ReadingQuality {
    Ok,
    Suspect,
    Missing,
}

/// Caller input for one synth tick.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SynthEmitRequest {
    /// Tenant id stamped on every emitted row.
    pub tenant_id: String,
    /// Meter ids to emit for this tick. Order is preserved in the
    /// response.
    pub meters: Vec<String>,
    /// Tick epoch (UTC ms). Each row's `epoch_ms` derives from this
    /// (± jitter for the jitter-eligible meter).
    pub tick_epoch_ms: i64,
    /// Optional mess knobs. Missing fields fall back to env vars then
    /// defaults.
    #[serde(default)]
    pub knobs: SynthKnobs,
}

/// Mess-injection knobs. All probabilities are per-tick, per-meter
/// (where applicable).
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct SynthKnobs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spike_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stuck_prob: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jitter_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nan_prob: Option<f64>,
}

/// Per-tick observability counters returned alongside `rows`. Stage 01's
/// success bar reads these.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SynthStats {
    pub emitted: u32,
    pub gaps: u32,
    pub spikes: u32,
    pub stuck_active: u32,
    pub nans: u32,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SynthEmitResponse {
    pub rows: Vec<MeterReading>,
    pub stats: SynthStats,
}

/// Five-field descriptor for `rubix.dataflow.synth.emit`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Emit synthetic, deliberately-messy meter readings for one tick (energy + water).",
    when_to_use: concat!(
        "Use as the producer stage in the data-flow scenario, in load tests ",
        "of the warehouse ingest path, or anywhere a deterministic stream of ",
        "realistic-but-broken meter readings is needed."
    ),
    when_not_to_use: concat!(
        "Do not use to read real meters (this is a synth tool — no I/O). ",
        "Do not use to write rows to the warehouse — chain it into ",
        "rubix.warehouse.ingest from a flow."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"site-a\", \"meters\": [\"site-a.elec.main\"], ",
        "\"tick_epoch_ms\": 1748275200000, \"knobs\": { \"seed\": 42 } }\n",
        "Output: { \"rows\": [ { \"tenant_id\": \"site-a\", \"meter_id\": ",
        "\"site-a.elec.main\", \"kind\": \"electricity\", \"unit\": \"kWh\", ",
        "\"epoch_ms\": 1748275200000, \"value\": 10001.2, \"quality\": \"ok\" } ], ",
        "\"stats\": { \"emitted\": 1, \"gaps\": 0, \"spikes\": 0, ",
        "\"stuck_active\": 0, \"nans\": 0 } }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.warehouse.ingest",
            wins_when: "the caller is generating rows, not persisting them.",
        },
    ],
};
