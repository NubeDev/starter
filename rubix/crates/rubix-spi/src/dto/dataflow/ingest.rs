//! `rubix.warehouse.ingest` — request/response DTOs and tool descriptor.
//!
//! Accepts a batch of [`MeterReading`] rows matching stage 01's wire
//! shape and persists them into `rubix.meter_readings_raw` (L1, see
//! `rubix/docs/sessions/data-flow/02-ingest-l1.md`). Append-only: no
//! `Reversible` wiring — undo of a single raw row makes no sense at
//! this layer; operators undo at the mart layer (stage 03+).
//!
//! See `rubix/crates/rubix-agent/migrations/0003_meter_readings_raw/up.sql`
//! for the schema this writer targets.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::ToolDescriptor;
use crate::dto::dataflow::synth::MeterReading;

/// `starter-authz` permission string. Writers carry the same
/// `system.write` capability the other persistence verbs already
/// gate on; the dispatch wrapper calls `Authz::check` before invoke.
pub const REQUIRED_PERMISSION: &str = "system.write";

/// Caller input for one ingest batch.
///
/// The producer flow forwards the unaltered [`MeterReading`] vector
/// the upstream synth tool returned, so the shape on this seam is
/// **bit-for-bit identical** to `SynthEmitResponse::rows`. That
/// fidelity is the whole point of L1 — clean / downsample / shape
/// happens at L2 (stage 03), not here.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseIngestRequest {
    /// Tenant id stamped on the change envelope (the per-row
    /// `tenant_id` column comes from each [`MeterReading`]; this
    /// field is the *batch* tenant for the audit trail and is
    /// optional — the writer does not enforce per-row equality so a
    /// future replay tool can backfill mixed-tenant batches).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    /// The rows to land. Empty is a no-op (returns `inserted = 0`)
    /// so the producer flow's downstream `tool_call` short-circuit
    /// is harmless.
    #[serde(default)]
    pub rows: Vec<MeterReading>,
}

/// Tool reply. Carries the row count and the per-batch wall-clock
/// so dashboards can compute write latency without a second probe.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WarehouseIngestResponse {
    /// `rubix.warehouse.ingested` (rows ≥ 1) or
    /// `rubix.warehouse.ingest.empty` (rows == 0).
    pub summary: Diagnostic,
    /// Row count successfully sent to ClickHouse. Equal to
    /// `request.rows.len()` on success — the writer does not drop
    /// rows silently.
    pub inserted: u32,
    /// Wall-clock at which the INSERT returned, UTC ms.
    pub written_at_ms: i64,
}

/// Five-field descriptor for `rubix.warehouse.ingest`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose: "Persist a batch of synthetic or live meter readings into the L1 raw warehouse table.",
    when_to_use: concat!(
        "Use as the terminal stage in the data-flow producer flow ",
        "(after rubix.dataflow.synth.emit) or in any pipeline that needs ",
        "to land bit-for-bit faithful meter rows in ClickHouse without ",
        "transformation."
    ),
    when_not_to_use: concat!(
        "Do not use to write cleaned or downsampled rows — that is L2's ",
        "job (stage 03's mart ingest). Do not use to drop / replay rows ",
        "(append-only; no undo)."
    ),
    example: concat!(
        "Input:  { \"tenant_id\": \"site-a\", \"rows\": [ ",
        "{ \"tenant_id\": \"site-a\", \"meter_id\": \"site-a.elec.main\", ",
        "\"kind\": \"electricity\", \"unit\": \"kWh\", \"epoch_ms\": ",
        "1748275200000, \"value\": 10001.2, \"quality\": \"ok\" } ] }\n",
        "Output: { \"summary\": { \"code\": \"rubix.warehouse.ingested\" }, ",
        "\"inserted\": 1, \"written_at_ms\": 1748275200042 }"
    ),
    siblings: &[],
};
