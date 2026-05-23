//! `rubix.system.db` — request/response DTOs and tool descriptor.
//!
//! Reports database-engine reachability + engine-reported storage
//! usage (distinct from OS-level [`rubix.system.disk`](super::disk),
//! which reads the filesystem).
//!
//! v0 carries a stub probe; the real engine query lands when the
//! DB pool is wired. See
//! [docs/design/migrations/](../../../../docs/design/migrations/README.md)
//! for the boot-order plan.

use serde::{Deserialize, Serialize};
use starter_spi::i18n::Diagnostic;
use utoipa::ToSchema;

use crate::descriptor::{SiblingTool, ToolDescriptor};

/// Caller input for `rubix.system.db`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct DbHealthRequest {
    /// Optional DSN override. When absent, the agent probes the
    /// DSN it was booted with, falling back to an in-memory stub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dsn: Option<String>,
}

/// Tool reply. `summary` is the keyed outcome; the data fields carry
/// raw numbers a follow-up tool or human inspector may want.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DbHealthResponse {
    /// Outcome keyed for the transport layer to render.
    pub summary: Diagnostic,
    /// DSN that was probed (with credentials elided).
    pub dsn: String,
    /// Whether the engine answered the ping.
    pub reachable: bool,
    /// Bytes the engine reports as in-use across all schemas.
    pub used_bytes: u64,
    /// Epoch milliseconds (UTC) at which the probe ran.
    pub probed_at_ms: i64,
}

/// `starter-authz` permission string the caller must hold to invoke
/// this tool. The dispatch wrapper passes this to `Authz::check`
/// before any probe runs. See
/// [docs/design/auth/](../../../../docs/design/auth/README.md).
pub const REQUIRED_PERMISSION: &str = "system.read";

/// Five-field descriptor for `rubix.system.db`.
pub static DESCRIPTOR: ToolDescriptor = ToolDescriptor {
    purpose:
        "Report database engine reachability and engine-reported storage usage for the rubix Postgres instance.",
    when_to_use: concat!(
        "Use when the operator asks 'is the database up?' or 'how much DB ",
        "space are we using?', or when a write-path skill needs to confirm ",
        "the engine is alive before retrying."
    ),
    when_not_to_use: concat!(
        "Do not use to probe OS-level filesystem usage (call ",
        "rubix.system.disk instead). Do not use to read application ",
        "tables — this verb only probes engine health."
    ),
    example: concat!(
        "Input:  { }\n",
        "Output: { \"summary\": { \"code\": \"rubix.system.db.ok\", ",
        "\"params\": { \"used\": 12345678, \"at\": <epoch_ms> } }, ",
        "\"dsn\": \"sqlite::memory:\", \"reachable\": true, ",
        "\"used_bytes\": 12345678, \"probed_at_ms\": 1764892800000 }"
    ),
    siblings: &[
        SiblingTool {
            id: "rubix.system.disk",
            wins_when: "the caller wants OS-level filesystem usage rather than DB-engine bytes.",
        },
        SiblingTool {
            id: "rubix.system.flow_errors",
            wins_when: "the symptom is flow misbehaviour, not DB unreachability.",
        },
    ],
};
