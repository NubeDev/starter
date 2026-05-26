//! `rubix.warehouse.clean_minute` — request/response DTOs and tool
//! descriptor.
//!
//! Materialises 1-minute buckets from L1 (`rubix.meter_readings_raw`)
//! into L2 (`rubix.meter_readings_1m`) over a lookback window.
//! Called once per minute by the bundled `com.rubix.data-flow.cleaner`
//! flow. Append-only at the row level; idempotent because L2 is
//! `ReplacingMergeTree` (re-running the same bucket supersedes).
//! See `rubix/docs/sessions/data-flow/03-clean-to-l2.md`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// `starter-authz` permission string. Mirrors `warehouse.ingest`:
/// the cleaner writes rows on the warehouse side, so it carries
/// the same `system.write` capability.
pub const REQUIRED_PERMISSION: &str = "system.write";

/// Default lookback for one cleaner pass, in minutes. Five minutes
/// covers two cleaner ticks of slop plus the producer's claim
/// cadence — wide enough to refine late-arriving L1 rows, narrow
/// enough that the per-tick SQL stays cheap.
pub const DEFAULT_LOOKBACK_MINUTES: u32 = 5;

/// Caller input. All knobs default; the cleaner flow YAML can
/// pass an empty body and rely on the defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct WarehouseCleanMinuteRequest {
    /// Lookback window, in minutes, ending at `now() - 1 minute`
    /// (the current minute is incomplete and never cleaned).
    /// Defaults to [`DEFAULT_LOOKBACK_MINUTES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookback_minutes: Option<u32>,
}

/// Tool reply. Carries the bucket count materialised and the
/// per-pass wall-clock so dashboards can compute cleaner lag
/// without a second probe.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseCleanMinuteResponse {
    /// `rubix.warehouse.cleaned` (rows >= 1) or
    /// `rubix.warehouse.clean.empty` (rows == 0, i.e. lookback
    /// window held no L1 data and no synthetic meters).
    pub summary: Diagnostic,
    /// Total L2 row count INSERTed by this pass. Includes
    /// `quality='missing'` calendar fills, so a healthy run with
    /// three meters and a 5-minute lookback emits ~15 rows per
    /// tick.
    pub rows: u32,
    /// Lookback window actually used (after default substitution).
    pub lookback_minutes: u32,
    /// Wall-clock at which the INSERT returned, UTC ms.
    pub written_at_ms: i64,
}

/// Five-field descriptor for `rubix.warehouse.clean_minute`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Materialise 1-minute cleaned buckets from L1 raw meter readings into the L2 mart.",
    when_to_use: concat!(
        "Use as the only step in the bundled `com.rubix.data-flow.cleaner` ",
        "flow's scheduled tick (once per minute). The query is idempotent ",
        "over its lookback window thanks to L2's ReplacingMergeTree engine."
    ),
    when_not_to_use: concat!(
        "Do not use as a backfill verb for historical ranges — the lookback ",
        "is bounded and the rolling-median window is small. Backfill ",
        "belongs in a separate verb that takes an explicit `[from, to]`. ",
        "Do not use to write raw rows (that is `rubix.warehouse.ingest`)."
    ),
    example: concat!(
        "Input:  { \"lookback_minutes\": 5 }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.cleaned\" }, ",
        "\"rows\": 15, \"lookback_minutes\": 5, \"written_at_ms\": 1748275200042 }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.warehouse.ingest",
        wins_when: "L1 raw rows are already landing — clean_minute reshapes them into L2 buckets",
    }],
};
