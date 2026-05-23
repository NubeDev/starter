//! `rubix.system.flow_errors` — request/response DTOs and tool descriptor.
//!
//! Reports the count and a small sample of recent flow execution
//! errors. v0 reads from an in-process registry handle passed at
//! tool construction; once flow persistence is wired the probe
//! queries the audit projection instead. See
//! [docs/design/audit/](../../../../docs/design/audit/README.md).

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.system.flow_errors`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct FlowErrorsRequest {
    /// Look-back window in seconds. Defaults to 3600 (one hour).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_secs: Option<u32>,
}

/// One captured error sample.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowErrorSample {
    /// Flow id the error came from.
    pub flow_id: String,
    /// Short error message (truncated to 200 chars upstream).
    pub message: String,
    /// Epoch milliseconds (UTC) at which the error fired.
    pub at_ms: i64,
}

/// Tool reply.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FlowErrorsResponse {
    /// Outcome keyed for the transport layer to render.
    pub summary: Diagnostic,
    /// Look-back window actually used.
    pub window_secs: u32,
    /// Total errors observed in the window.
    pub error_count: u32,
    /// Up to 10 most-recent error samples; oldest first.
    pub samples: Vec<FlowErrorSample>,
    /// Epoch milliseconds (UTC) at which the probe ran.
    pub probed_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold to invoke
/// this tool. The dispatch wrapper passes this to `Authz::check`
/// before any probe runs. See
/// [docs/design/auth/](../../../../docs/design/auth/README.md).
pub const REQUIRED_PERMISSION: &str = "system.read";

/// Threshold (errors-in-window) above which the summary switches to
/// `rubix.system.flow_errors.warn`.
pub const WARN_THRESHOLD: u32 = 1;

/// Threshold (errors-in-window) above which the summary switches to
/// `rubix.system.flow_errors.error`.
pub const ERROR_THRESHOLD: u32 = 10;

/// Five-field descriptor for `rubix.system.flow_errors`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Report how many flow executions have errored in a recent time window on the rubix-agent host.",
    when_to_use: concat!(
        "Use when the operator asks 'are flows healthy?' or 'have any flows ",
        "errored recently?', or when a system-check skill is triaging a ",
        "broader 'something is wrong' signal."
    ),
    when_not_to_use: concat!(
        "Do not use to fetch full flow execution traces (that requires a ",
        "flow-ops verb). Do not use to count user-facing alerts (call ",
        "rubix.alert.send for the alert sink instead)."
    ),
    example: concat!(
        "Input:  { \"window_secs\": 3600 }\n",
        "Output: { \"summary\": { \"code\": \"rubix.system.flow_errors.warn\", ",
        "\"params\": { \"count\": 3, \"window\": 3600, \"at\": <epoch_ms> } }, ",
        "\"window_secs\": 3600, \"error_count\": 3, \"samples\": [...], ",
        "\"probed_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.alert.send",
            wins_when: "the caller wants to emit a new alert, not count past errors.",
        },
        SiblingTool {
            id: "rubix.system.db",
            wins_when: "the suspected root cause is the database, not the flow engine.",
        },
    ],
};
