//! `rubix.warehouse.rollup_15m` — request/response DTOs and tool
//! descriptor.
//!
//! Materialises 15-minute buckets from L2 (`rubix.meter_readings_1m`)
//! into L3 (`rubix.meter_readings_15m`) over a lookback window.
//! Called once per ~5 minutes by the bundled
//! `com.rubix.data-flow.rollup` flow. Append-only at the row
//! level; idempotent because L3 is `ReplacingMergeTree`
//! (re-running the same bucket supersedes).
//!
//! See `rubix/docs/sessions/data-flow/05-dashboard-at-scale.md`.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// `starter-authz` permission string. Mirrors
/// `warehouse.clean_minute`: the rollup writes rows on the
/// warehouse side, so it carries the same `system.write`
/// capability.
pub const REQUIRED_PERMISSION: &str = "system.write";

/// Default lookback for one rollup pass, in minutes. Thirty
/// minutes covers two rollup ticks of slop plus the cleaner's
/// own 5-minute lag — wide enough to refine the most recent two
/// 15-minute buckets, narrow enough that the per-tick SQL stays
/// cheap.
pub const DEFAULT_LOOKBACK_MINUTES: u32 = 30;

/// Bucket width in minutes. Locked to 15 so the table name and
/// the SQL match; surfaced as a const so tests and the SQL
/// builder agree.
pub const BUCKET_MINUTES: u32 = 15;

/// Caller input. All knobs default; the rollup flow YAML can
/// pass an empty body and rely on the defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct WarehouseRollup15mRequest {
    /// Lookback window, in minutes, ending at the last fully-
    /// elapsed 15-minute bucket. Defaults to
    /// [`DEFAULT_LOOKBACK_MINUTES`]. Wider windows are allowed for
    /// one-shot backfills (see [`MAX_LOOKBACK_MINUTES`] in the
    /// tool impl).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookback_minutes: Option<u32>,
}

/// Tool reply. Carries the bucket count materialised and the
/// per-pass wall-clock so dashboards can compute rollup lag
/// without a second probe.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseRollup15mResponse {
    /// `rubix.warehouse.rolled_up` (rows >= 1) or
    /// `rubix.warehouse.rollup.empty` (rows == 0, i.e. lookback
    /// window held no L2 data).
    pub summary: Diagnostic,
    /// Total L3 row count INSERTed by this pass.
    pub rows: u32,
    /// Lookback window actually used (after default substitution).
    pub lookback_minutes: u32,
    /// Wall-clock at which the INSERT returned, UTC ms.
    pub written_at_ms: i64,
}

/// Five-field descriptor for `rubix.warehouse.rollup_15m`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Materialise 15-minute downsampled buckets from L2 cleaned meter readings into the L3 dashboard mart.",
    when_to_use: concat!(
        "Use as the only step in the bundled `com.rubix.data-flow.rollup` ",
        "flow's scheduled tick (once per ~5 minutes). The query is ",
        "idempotent over its lookback window thanks to L3's ",
        "ReplacingMergeTree engine. Dashboards reading 30-day windows ",
        "select from L3 so the browser never streams L2."
    ),
    when_not_to_use: concat!(
        "Do not use as a backfill verb for historical ranges spanning ",
        "weeks — the lookback is bounded by the tool's MAX_LOOKBACK_MINUTES. ",
        "Do not use to compute the cleaned L2 buckets (that is ",
        "`rubix.warehouse.clean_minute`). Do not use to read L3 (that ",
        "is `rubix.analytics.query` against the named L3 templates)."
    ),
    example: concat!(
        "Input:  { \"lookback_minutes\": 30 }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.rolled_up\" }, ",
        "\"rows\": 6, \"lookback_minutes\": 30, \"written_at_ms\": 1748275200042 }"
    ),
    siblings: &[SiblingTool {
        id: "rubix.warehouse.clean_minute",
        wins_when: "the goal is to materialise the upstream L2 cleaned buckets, not the L3 downsampled ones",
    }],
};
