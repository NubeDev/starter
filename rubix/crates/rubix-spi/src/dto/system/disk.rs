//! `rubix.system.disk` — request/response DTOs and tool descriptor.
//!
//! DTOs are `utoipa::ToSchema`-derived; the descriptor is a
//! `&'static` value (anti-prompt-injection parity with skill
//! bundles). See [docs/design/mcp-ux/](../../../../docs/design/mcp-ux/README.md)
//! for the five-field descriptor contract.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.system.disk`.
///
/// All fields optional; an empty request reads the disk hosting
/// the agent's current working directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DiskUsageRequest {
    /// Filesystem mount point to probe (e.g. `"/"`, `"/var"`). When
    /// absent, the agent picks the disk containing its CWD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount: Option<String>,
}

/// Tool reply.
///
/// `summary` is the keyed, transport-localisable outcome; `data`
/// carries the raw numbers an upstream tool or human inspector
/// might want. Per [docs/design/i18n-prefs/](../../../../docs/design/i18n-prefs/README.md),
/// tool outputs are `Diagnostic` + structured data, never strings.
/// Timestamps travel as `i64` epoch milliseconds (UTC); the
/// transport renders them against the caller's timezone +
/// date_format + time_format.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiskUsageResponse {
    /// Outcome keyed for the transport layer to render.
    pub summary: Diagnostic,
    /// Probed mount point.
    pub mount: String,
    /// Total bytes on the filesystem.
    pub total_bytes: u64,
    /// Free bytes on the filesystem.
    pub free_bytes: u64,
    /// Percent used (0–100, rounded to nearest integer).
    pub percent_used: u8,
    /// Epoch milliseconds (UTC) at which the probe ran.
    pub probed_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold to invoke
/// this tool. The dispatch wrapper passes this to `Authz::check`
/// before any probe runs. See
/// [docs/design/auth/](../../../../docs/design/auth/README.md).
pub const REQUIRED_PERMISSION: &str = "system.read";

/// Threshold (percent used) above which the summary code switches
/// from `rubix.system.disk.ok` to `rubix.system.disk.warn`.
pub const WARN_THRESHOLD: u8 = 80;

/// Threshold (percent used) above which the summary code switches
/// from `rubix.system.disk.warn` to `rubix.system.disk.full`.
pub const FULL_THRESHOLD: u8 = 95;

/// Five-field descriptor for `rubix.system.disk`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Report disk usage for a filesystem mount point on the rubix-agent host.",
    when_to_use: concat!(
        "Use when the operator asks 'how full is the disk?', when an agent ",
        "is investigating a degraded system, or as the first step in a ",
        "system-check skill."
    ),
    when_not_to_use: concat!(
        "Do not use to probe a remote host (this reads the agent's local ",
        "filesystem only). Do not use as a substitute for ",
        "rubix.system.db (database disk usage is reported by the DB ",
        "engine, not the filesystem)."
    ),
    example: concat!(
        "Input:  { \"mount\": \"/\" }\n",
        "Output: { \"summary\": { \"code\": \"rubix.system.disk.warn\", ",
        "\"params\": { \"percent\": 89, \"free\": 125000000000, ",
        "\"at\": <epoch_ms> } }, ",
        "\"mount\": \"/\", \"total_bytes\": 1000000000000, ",
        "\"free_bytes\": 125000000000, \"percent_used\": 88, ",
        "\"probed_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.system.db",
            wins_when: "the caller wants OS-level disk usage rather than DB-engine bytes.",
        },
        SiblingTool {
            id: "rubix.system.flow_errors",
            wins_when: "the symptom is storage exhaustion, not flow misbehaviour.",
        },
    ],
};
